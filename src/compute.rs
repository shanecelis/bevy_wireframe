use bevy::{
    log::{info, trace},
    app::{App, Plugin},
    asset::{embedded_asset, DirectAssetAccessExt, Handle, Asset},
    ecs::{
        component::Component,
        entity::Entity,
        query::{QueryState, With},
        schedule::IntoSystemConfigs,
        system::{lifetimeless::SRes, Commands, Query, Res, ResMut, Resource, SystemParamItem},
        world::{FromWorld, World},
    },
    log::warn,
    math::Vec4,
    prelude::{Deref, DerefMut},
    render::{
        mesh::{GpuMesh, Mesh, VertexAttributeValues},
        render_asset::{PrepareAssetError, RenderAssetUsages, RenderAssets},
        render_asset::{RenderAsset, RenderAssetPlugin},
        render_graph::{
            Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel, SlotInfo, SlotType,
        },
        render_resource::{
            binding_types::{storage_buffer, storage_buffer_read_only},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, Buffer,
            BufferDescriptor, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
            ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, Shader, ShaderStages,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
    sprite::{Mesh2dHandle, RenderMesh2dInstance},
};

use crate::wireframe2d::{WireframeMesh2d, WireframeMesh2dInstances};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct FaceLabel;

#[derive(Component)]
pub struct FaceBinding {
    bind_group: BindGroup,
    vertex_count: usize,
    dist_buffer: Buffer,
}

#[derive(Resource)]
pub struct FacePipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

pub struct PosBuffer {
    pub buffer: Buffer,
    pub vertex_count: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct FaceBuffer {
    buffer: Buffer,
}
use crate::material::WireframeMaterial;

pub struct FacePlugin;
pub struct FacePlugin2d;

impl Plugin for FacePlugin2d {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "face_compute.wgsl");
        app.add_plugins(RenderAssetPlugin::<PosBuffer, GpuImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        let node = FaceComputeNode::from_world(render_app.world_mut());
        render_app
            .add_systems(
            Render,
            (
                prepare_face_buffers2d.in_set(RenderSet::PrepareResources),
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            ),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(FaceLabel, node);
        render_graph.add_node_edge(FaceLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.sub_app_mut(RenderApp).init_resource::<FacePipeline>();
    }
}

impl Plugin for FacePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "face_compute.wgsl");
        app.add_plugins(RenderAssetPlugin::<PosBuffer, GpuImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        let node = FaceComputeNode::from_world(render_app.world_mut());
        render_app
            .add_systems(
            Render,
            (
                prepare_face_buffers.in_set(RenderSet::PrepareResources),
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            ),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(FaceLabel, node);
        render_graph.add_node_edge(FaceLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.sub_app_mut(RenderApp).init_resource::<FacePipeline>();
    }
}

fn prepare_face_buffers2d(
    mut commands: Commands,
    meshes: Res<RenderAssets<GpuMesh>>,
    query: Query<(Entity, &Mesh2dHandle)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, handle) in &query {
        let mesh_asset_id = handle.0.id();
        let Some(gpu_mesh) = meshes.get(mesh_asset_id) else {
            warn!("no gpu mesh");
            continue;
        };
        let vertex_count = gpu_mesh.vertex_count as usize;
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("face_compute"),
            size: (std::mem::size_of::<Vec4>() * vertex_count / 3) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        trace!("make face buffer 2d");
        commands.entity(entity).insert(FaceBuffer { buffer });
    }
}

fn prepare_face_buffers(
    mut commands: Commands,
    meshes: Res<RenderAssets<GpuMesh>>,
    query: Query<(Entity, &Handle<Mesh>)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, handle) in &query {
        let mesh_asset_id = handle.id();
        let Some(gpu_mesh) = meshes.get(mesh_asset_id) else {
            warn!("no gpu mesh");
            continue;
        };
        let vertex_count = gpu_mesh.vertex_count as usize;
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("face_compute"),
            size: (std::mem::size_of::<Vec4>() * vertex_count / 3) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        trace!("make face buffer");
        commands.entity(entity).insert(FaceBuffer { buffer });
    }
}

impl RenderAsset for PosBuffer {
    type SourceAsset = Mesh;
    type Param = (SRes<RenderDevice>,);

    /// We add MAIN_WORLD to usage because we need the mesh to be accessible in
    /// main world for at least the initialization. It would be nice to not have
    /// to do this.
    #[inline]
    fn asset_usage(mesh: &Self::SourceAsset) -> RenderAssetUsages {
        mesh.asset_usage | RenderAssetUsages::MAIN_WORLD
    }

    fn byte_len(mesh: &Self::SourceAsset) -> Option<usize> {
        Some(mesh.count_vertices() * std::mem::size_of::<Vec4>())
    }

    fn prepare_asset(
        mesh: Self::SourceAsset,
        (render_device,): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            warn!("no position vertices");
            return Err(PrepareAssetError::RetryNextUpdate(mesh));
        };
        let v_pos_4: Vec<[f32; 4]> = positions.iter().map(|x| crate::pad(*x)).collect();

        let vertex_count = mesh.count_vertices();

        let pos_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("pos_buffer"),
            contents: bytemuck::cast_slice(v_pos_4.as_slice()),
            usage: BufferUsages::STORAGE,
        });
        info!("make pos buffer");

        Ok(PosBuffer {
            vertex_count,
            buffer: pos_buffer,
        })
    }
}

impl FromWorld for FacePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "ScreenSpaceDist",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer_read_only::<Vec<Vec4>>(false),
                    storage_buffer::<Vec<Vec4>>(false),
                ),
            ),
        );

        let shader_defs = vec!["MODEL_DIST".into()];
        let shader = world.load_asset::<Shader>("embedded://bevy_wireframe/face_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Face compute shader".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs,
            entry_point: "main".into(),
        });
        FacePipeline { layout, pipeline }
    }
}

pub struct FaceComputeNode {
    query: QueryState<&'static FaceBinding>,
}

impl FromWorld for FaceComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<FacePipeline>,
    render_device: Res<RenderDevice>,
    pos_buffers: Res<RenderAssets<PosBuffer>>,
    wireframe_mesh: Query<(Entity, &FaceBuffer, &Mesh2dHandle)>,
) {
    for (entity, dist_buffer, handle) in wireframe_mesh.iter() {

        let mesh_asset_id = handle.0.id();
        let Some(pos_buffer) = pos_buffers.get(mesh_asset_id) else {
            warn!("no pos buffer");
            return;
        };
        trace!("start bind group");
        let bind_group = render_device.create_bind_group(
            None,
            &pipeline.layout,
            &BindGroupEntries::sequential((
                pos_buffer.buffer.as_entire_buffer_binding(),
                dist_buffer.buffer.as_entire_buffer_binding(),
            )),
        );
        trace!("end bind group");
        let vertex_count = pos_buffer.vertex_count;
        commands.entity(entity).insert(FaceBinding {
            bind_group,
            vertex_count,
            dist_buffer: dist_buffer.buffer.clone(),
        });
    }
}

impl Node for FaceComputeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("face", SlotType::Buffer)]
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for wireframe_binding in self.query.iter_manual(world) {
            let bind_group = &wireframe_binding.bind_group;
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline = world.resource::<FacePipeline>();

            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            let update_pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.pipeline)
                .unwrap();
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups((wireframe_binding.vertex_count / 3) as u32, 1, 1);
            graph.set_output("face", wireframe_binding.dist_buffer.clone())?;
        }
        Ok(())
    }
}
