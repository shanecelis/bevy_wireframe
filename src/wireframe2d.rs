use bevy::{
    app::{App, Plugin},
    asset::{embedded_asset, AssetServer, DirectAssetAccessExt, Handle},
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

#[derive(Component, Default)]
pub struct WireframeMesh2d;

#[derive(Resource)]
pub struct WireframeMesh2dPipeline {
    /// this pipeline wraps the standard [`Mesh2dPipeline`]
    mesh2d_pipeline: Mesh2dPipeline,
    shader: Handle<Shader>,
    wireframe2d_layout: BindGroupLayout,
}

impl FromWorld for WireframeMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load::<Shader>("embedded://bevy_wireframe/wireframe.wgsl");
        let wireframe2d_layout = render_device.create_bind_group_layout(
            "Tri",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                (storage_buffer_read_only::<Vec<Vec4>>(false),),
            ),
        );
        Self {
            mesh2d_pipeline: Mesh2dPipeline::from_world(world),
            shader,
            wireframe2d_layout,
        }
    }
}

fn pad(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}

// We implement `SpecializedPipeline` to customize the default rendering from `Mesh2dPipeline`
impl SpecializedMeshPipeline for WireframeMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key, layout)?;

        // Customize how to store the meshes' vertex attributes in the vertex buffer
        // descriptor.vertex.buffers.push(VertexBufferLayout {
        //     array_stride: std::mem::size_of::<Vec4>() as u64,
        //     step_mode: VertexStepMode::Vertex,
        //     attributes: vec![VertexAttribute {
        //         format: VertexFormat::Float32x4,
        //         offset: 0,
        //         shader_location: 10, // shader locations 0-2 are taken up by Position, Normal and UV attributes
        //     }],
        // });
        descriptor.layout.push(self.wireframe2d_layout.clone());

        descriptor.vertex.shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.label = Some("wireframe_mesh2d_pipeline".into());
        Ok(descriptor)
    }
}

// pub struct SetDistVertexBuffer<const I: usize>;
// impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetDistVertexBuffer<I> {
//     type Param = ();
//     type ViewQuery = ();
//     type ItemQuery = Read<DistBuffer>;

//     #[inline]
//     fn render<'w>(
//         _item: &P,
//         _view: (),
//         dist_buffer: Option<&'w DistBuffer>,
//         _mesh2d_bind_group: SystemParamItem<'w, '_, Self::Param>,
//         pass: &mut TrackedRenderPass<'w>,
//     ) -> RenderCommandResult {
//         let Some(dist_buffer) = dist_buffer else {
//             warn!("no dist");
//             return RenderCommandResult::Failure;
//         };
//         pass.set_vertex_buffer(I, dist_buffer.buffer.slice(..));
//         RenderCommandResult::Success
//     }
// }

// This specifies how to render a colored 2d mesh
type DrawWireframeMesh2d = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetMesh2dBindGroup<1>,
    // Set the dist buffer as vertex buffer 1
    SetTriBindGroup<2>,
    // Draw the mesh
    DrawMesh2d,
    // XXX: This was our complicated way of setting the DistVertexBuffer.
    // WireframeDrawMesh2d,
);

/// Plugin that renders [`WireframeMesh2d`]s
pub struct WireframeMesh2dPlugin;

/// Our custom pipeline needs its own instance storage
#[derive(Resource, Deref, DerefMut, Default)]
pub struct WireframeMesh2dInstances(EntityHashMap<Entity, RenderMesh2dInstance>);

impl Plugin for WireframeMesh2dPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "wireframe.wgsl");
        embedded_asset!(app, "dist.wgsl");
        app.add_plugins(RenderAssetPlugin::<PosBuffer, GpuImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        let node = ScreenspaceDistNode::from_world(render_app.world_mut());
        // Register our custom draw function, and add our render systems
        render_app
            .add_render_command::<Transparent2d, DrawWireframeMesh2d>()
            .init_resource::<SpecializedMeshPipelines<WireframeMesh2dPipeline>>()
            .init_resource::<WireframeMesh2dInstances>()
            .add_systems(
                ExtractSchedule,
                extract_wireframe_mesh2d.after(extract_mesh2d),
            )
            .add_systems(
                Render,
                (
                    prepare_dist_buffers.in_set(RenderSet::PrepareResources),
                    prepare_wireframe2d_bind_group.in_set(RenderSet::PrepareBindGroups),
                    prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_systems(
                Render,
                queue_wireframe_mesh2d.in_set(RenderSet::QueueMeshes),
            );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ScreenSpaceDistLabel, node);
        render_graph.add_node_edge(ScreenSpaceDistLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.sub_app_mut(RenderApp)
            .init_resource::<WireframeMesh2dPipeline>()
            .init_resource::<ScreenspaceDistPipeline>();
    }
}

/// Extract the [`WireframeMesh2d`] marker component into the render app
#[allow(clippy::type_complexity)]
pub fn extract_wireframe_mesh2d(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    // When extracting, you must use `Extract` to mark the `SystemParam`s
    // which should be taken from the main world.
    query: Extract<
        Query<(Entity, &ViewVisibility, &GlobalTransform, &Mesh2dHandle), With<WireframeMesh2d>>,
    >,
    mut wireframe_mesh_instances: ResMut<WireframeMesh2dInstances>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, view_visibility, transform, handle) in &query {
        if !view_visibility.get() {
            continue;
        }

        let transforms = Mesh2dTransforms {
            transform: (&transform.affine()).into(),
            flags: MeshFlags::empty().bits(),
        };

        values.push((entity, (handle.clone(), WireframeMesh2d)));

        let mesh_asset_id = handle.0.id();
        if !wireframe_mesh_instances.contains_key(&entity) {
            wireframe_mesh_instances.insert(
                entity,
                RenderMesh2dInstance {
                    mesh_asset_id,
                    transforms,
                    material_bind_group_id: Material2dBindGroupId::default(),
                    automatic_batching: false,
                },
            );
        }
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

/// Queue the 2d meshes marked with [`WireframeMesh2d`] using our custom pipeline and draw function
#[allow(clippy::too_many_arguments)]
pub fn queue_wireframe_mesh2d(
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    wireframe_mesh2d_pipeline: Res<WireframeMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WireframeMesh2dPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<GpuMesh>>,
    wireframe_mesh_instances: Res<WireframeMesh2dInstances>,
    mut views: Query<(
        &VisibleEntities,
        &mut SortedRenderPhase<Transparent2d>,
        &ExtractedView,
    )>,
) {
    if wireframe_mesh_instances.is_empty() {
        return;
    }
    // Iterate each view (a camera is a view)
    for (visible_entities, mut transparent_phase, view) in &mut views {
        let draw_wireframe_mesh2d = transparent_draw_functions
            .read()
            .id::<DrawWireframeMesh2d>();

        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        // Queue all entities visible to that view
        for visible_entity in visible_entities.iter::<WithMesh2d>() {
            if let Some(mesh_instance) = wireframe_mesh_instances.get(visible_entity) {
                let mesh2d_handle = mesh_instance.mesh_asset_id;
                let mesh2d_transforms = &mesh_instance.transforms;
                // Get our specialized pipeline
                let mut mesh2d_key = mesh_key;
                let Some(mesh) = render_meshes.get(mesh2d_handle) else {
                    warn!("No mesh");
                    continue;
                };

                mesh2d_key |= Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology());
                if !matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList) {
                    panic!(
                        "Expected a TriangleList but got {:?}",
                        mesh.primitive_topology()
                    );
                }
                let pipeline_id = pipelines
                    .specialize(
                        &pipeline_cache,
                        &wireframe_mesh2d_pipeline,
                        mesh2d_key,
                        &mesh.layout,
                    )
                    .expect("specialize 2d pipeline");

                let mesh_z = mesh2d_transforms.transform.translation.z;
                transparent_phase.add(Transparent2d {
                    entity: *visible_entity,
                    draw_function: draw_wireframe_mesh2d,
                    pipeline: pipeline_id,
                    // The 2d render items are sorted according to their z value before rendering,
                    // in order to get correct transparency
                    sort_key: FloatOrd(mesh_z),
                    // This material is not batched
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::NONE,
                });
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ScreenSpaceDistLabel;

#[derive(Component)]
struct WireframeBinding {
    bind_group: BindGroup,
    vertex_count: usize,
    dist_buffer: Buffer,
}

#[derive(Component)]
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

struct PosBuffer {
    buffer: Buffer,
    vertex_count: usize,
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
        let v_pos_4: Vec<[f32; 4]> = positions.iter().map(|x| pad(*x)).collect();

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

#[derive(Resource)]
struct ScreenspaceDistPipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
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

struct ScreenspaceDistNode {
    query: QueryState<&'static WireframeBinding>,
}

impl FromWorld for ScreenspaceDistNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
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

#[derive(Component)]
pub struct Wireframe2dBindGroup(BindGroup);

pub fn prepare_wireframe2d_bind_group(
    mut commands: Commands,
    pipeline: Res<WireframeMesh2dPipeline>,
    render_device: Res<RenderDevice>,
    query: Query<(Entity, &DistBuffer)>,
) {
    for (entity, dist_buffer) in query.iter() {
        commands
            .entity(entity)
            .insert(Wireframe2dBindGroup(render_device.create_bind_group(
                "wireframe2d_bind_group",
                &pipeline.wireframe2d_layout,
                &BindGroupEntries::single(dist_buffer.buffer.as_entire_buffer_binding()),
            )));
    }
}

pub struct SetTriBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetTriBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<Wireframe2dBindGroup>;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        bind_group: Option<&'w Wireframe2dBindGroup>,
        _mesh2d_bind_group: (),
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let Some(dynamic_offset) = item.extra_index().as_dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
            offset_count += 1;
        }
        let Some(bind_group) = bind_group else {
            warn!("no bind group");
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &bind_group.0, &dynamic_offsets[..offset_count]);
        RenderCommandResult::Success
    }
}
