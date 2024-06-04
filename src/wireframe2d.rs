use crate::compute::*;
use bevy::{
    app::{App, Plugin},
    asset::{embedded_asset, DirectAssetAccessExt, Handle},
    core_pipeline::core_2d::Transparent2d,
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        schedule::IntoSystemConfigs,
        system::{lifetimeless::Read, Commands, Local, Query, Res, ResMut, Resource},
        world::{FromWorld, World},
    },
    log::warn,
    math::{FloatOrd, Vec4},
    prelude::{Deref, DerefMut},
    render::{
        mesh::{GpuMesh, MeshVertexBufferLayoutRef},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, SortedRenderPhase, TrackedRenderPass,
            ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::storage_buffer_read_only, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, PipelineCache, PrimitiveTopology, RenderPipelineDescriptor,
            Shader, ShaderStages, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            SpecializedMeshPipelines,
        },
        renderer::RenderDevice,
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
        let shader = world.load_asset::<Shader>("embedded://bevy_wireframe/wireframe.wgsl");
        let wireframe2d_layout = render_device.create_bind_group_layout(
            "Face",
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

// We implement `SpecializedPipeline` to customize the default rendering from `Mesh2dPipeline`
impl SpecializedMeshPipeline for WireframeMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key, layout)?;
        descriptor.layout.push(self.wireframe2d_layout.clone());
        descriptor.vertex.shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.label = Some("wireframe_mesh2d_pipeline".into());
        Ok(descriptor)
    }
}

// This specifies how to render a colored 2d mesh
type DrawWireframeMesh2d = (
    // Set the pipeline
    SetItemPipeline,
    // Set the view uniform as bind group 0
    SetMesh2dViewBindGroup<0>,
    // Set the mesh uniform as bind group 1
    SetMesh2dBindGroup<1>,
    // Set the face buffer as bind group 2
    SetFaceBindGroup<2>,
    // Draw the mesh
    DrawMesh2d,
);

/// Plugin that renders [`WireframeMesh2d`]s
pub struct WireframeMesh2dPlugin;

/// Our custom pipeline needs its own instance storage
#[derive(Resource, Deref, DerefMut, Default)]
pub struct WireframeMesh2dInstances(EntityHashMap<Entity, RenderMesh2dInstance>);

impl Plugin for WireframeMesh2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::compute::FacePlugin);
        embedded_asset!(app, "wireframe.wgsl");

        let render_app = app.sub_app_mut(RenderApp);
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
                (prepare_wireframe2d_bind_group.in_set(RenderSet::PrepareBindGroups),),
            )
            .add_systems(
                Render,
                queue_wireframe_mesh2d.in_set(RenderSet::QueueMeshes),
            );
    }

    fn finish(&self, app: &mut App) {
        // Register our custom pipeline
        app.sub_app_mut(RenderApp)
            .init_resource::<WireframeMesh2dPipeline>();
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
            world_from_local: (&transform.affine()).into(),
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
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(Entity, &VisibleEntities, &ExtractedView)>,
) {
    if wireframe_mesh_instances.is_empty() {
        return;
    }
    // Iterate each view (a camera is a view)
    for (view_entity, visible_entities, view) in &mut views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };
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

                let mesh_z = mesh2d_transforms.world_from_local.translation.z;
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

#[derive(Component)]
pub struct Wireframe2dBindGroup(BindGroup);

pub fn prepare_wireframe2d_bind_group(
    mut commands: Commands,
    pipeline: Res<WireframeMesh2dPipeline>,
    render_device: Res<RenderDevice>,
    query: Query<(Entity, &FaceBuffer)>,
) {
    for (entity, face_buffer) in query.iter() {
        commands
            .entity(entity)
            .insert(Wireframe2dBindGroup(render_device.create_bind_group(
                "wireframe2d_bind_group",
                &pipeline.wireframe2d_layout,
                &BindGroupEntries::single(face_buffer.as_entire_buffer_binding()),
            )));
    }
}

pub struct SetFaceBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetFaceBindGroup<I> {
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
