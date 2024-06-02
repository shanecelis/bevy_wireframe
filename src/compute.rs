use bevy::{
    app::{App, Plugin},
    asset::{embedded_asset, DirectAssetAccessExt, Handle},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        query::{QueryState, With},
        schedule::IntoSystemConfigs,
        system::{
            lifetimeless::{Read, SRes},
            Commands, Local, Query, Res, ResMut, Resource, SystemParamItem,
        },
        world::{FromWorld, World},
    },
    log::warn,
    math::{FloatOrd, Vec4},
    prelude::{Deref, DerefMut},
    render::{
        mesh::{GpuMesh, Mesh, MeshVertexBufferLayoutRef, VertexAttributeValues},
        render_asset::{PrepareAssetError, RenderAssetUsages, RenderAssets},
        render_asset::{RenderAsset, RenderAssetPlugin},
        render_graph::{self, RenderGraph, RenderLabel, SlotInfo, SlotType},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, SortedRenderPhase, TrackedRenderPass,
        },
        render_resource::{
            binding_types::{storage_buffer, storage_buffer_read_only},
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, Buffer,
            BufferDescriptor, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
            ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, PrimitiveTopology,
            RenderPipelineDescriptor, Shader, ShaderStages, SpecializedMeshPipeline,
            SpecializedMeshPipelineError, SpecializedMeshPipelines,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        view::{ExtractedView, Msaa, ViewVisibility, VisibleEntities},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    sprite::{
        extract_mesh2d, DrawMesh2d, Material2dBindGroupId, Mesh2dHandle, Mesh2dPipeline,
        Mesh2dPipelineKey, Mesh2dTransforms, MeshFlags, RenderMesh2dInstance, SetMesh2dBindGroup,
        SetMesh2dViewBindGroup, WithMesh2d,
    },
    transform::components::GlobalTransform,
    utils::EntityHashMap,
};

use crate::wireframe2d::{WireframeMesh2d, WireframeMesh2dInstances};

pub struct TriPlugin;

impl Plugin for TriPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "dist.wgsl");
        app.add_plugins(RenderAssetPlugin::<PosBuffer, GpuImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        let node = ScreenspaceDistNode::from_world(render_app.world_mut());
        // Register our custom draw function, and add our render systems
        render_app
            // .add_systems(
            //     ExtractSchedule,
            //     extract_wireframe_mesh2d.after(extract_mesh2d),
            // )
            .add_systems(
                Render,
                (
                    prepare_dist_buffers.in_set(RenderSet::PrepareResources),
                    prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
                ),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ScreenSpaceDistLabel, node);
        render_graph.add_node_edge(ScreenSpaceDistLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.sub_app_mut(RenderApp)
            .init_resource::<ScreenspaceDistPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ScreenSpaceDistLabel;

#[derive(Component)]
pub struct WireframeBinding {
    bind_group: BindGroup,
    vertex_count: usize,
    dist_buffer: Buffer,
}


#[derive(Resource)]
pub struct ScreenspaceDistPipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

pub struct PosBuffer {
    pub buffer: Buffer,
    pub vertex_count: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct DistBuffer {
    buffer: Buffer,
}

fn prepare_dist_buffers(
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
            label: Some("dist"),
            size: (std::mem::size_of::<Vec4>() * vertex_count / 3) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        commands.entity(entity).insert(DistBuffer { buffer });
    }
}


impl RenderAsset for PosBuffer {
    type SourceAsset = Mesh;
    type Param = (SRes<RenderDevice>,);

    #[inline]
    fn asset_usage(mesh: &Self::SourceAsset) -> RenderAssetUsages {
        mesh.asset_usage
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

        Ok(PosBuffer {
            vertex_count,
            buffer: pos_buffer,
        })
    }
}


impl FromWorld for ScreenspaceDistPipeline {
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
        let shader = world.load_asset::<Shader>("embedded://bevy_wireframe/dist.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Wireframe compute shader".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs,
            entry_point: "main".into(),
        });
        ScreenspaceDistPipeline { layout, pipeline }
    }
}

pub struct ScreenspaceDistNode {
    query: QueryState<&'static WireframeBinding>,
}

impl FromWorld for ScreenspaceDistNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ScreenspaceDistPipeline>,
    render_device: Res<RenderDevice>,
    pos_buffers: Res<RenderAssets<PosBuffer>>,
    wireframe_mesh: Query<(Entity, &DistBuffer), With<WireframeMesh2d>>,
    mut wireframe_mesh_instances: ResMut<WireframeMesh2dInstances>,
) {
    for (entity, dist_buffer) in wireframe_mesh.iter() {
        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            wireframe_mesh_instances.get_mut(&entity)
        else {
            warn!("no wireframe mesh 2d");
            return;
        };
        let Some(pos_buffer) = pos_buffers.get(*mesh_asset_id) else {
            warn!("no pos buffer");
            return;
        };
        let bind_group = render_device.create_bind_group(
            None,
            &pipeline.layout,
            &BindGroupEntries::sequential((
                pos_buffer.buffer.as_entire_buffer_binding(),
                dist_buffer.buffer.as_entire_buffer_binding(),
            )),
        );
        let vertex_count = pos_buffer.vertex_count;
        commands.entity(entity).insert(WireframeBinding {
            bind_group,
            vertex_count,
            dist_buffer: dist_buffer.buffer.clone(),
        });
    }
}

impl render_graph::Node for ScreenspaceDistNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new("dist", SlotType::Buffer)]
    }

    fn run(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        for wireframe_binding in self.query.iter_manual(world) {
            let bind_group = &wireframe_binding.bind_group;
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline = world.resource::<ScreenspaceDistPipeline>();

            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            let update_pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.pipeline)
                .unwrap();
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups((wireframe_binding.vertex_count / 3) as u32, 1, 1);
            graph.set_output("dist", wireframe_binding.dist_buffer.clone())?;
        }
        Ok(())
    }
}
