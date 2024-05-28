/// minimal example of adding a custom render pipeline in bevy 0.11.
///
/// When this example runs, you should only see a blue screen. There are no
/// vertex buffers, or anything else in this example.  Effectively it is
/// shader-toy written in bevy.
///
/// This revision adds a post-processing node to the RenderGraph to
/// execute the shader.  Thanks to @Jasmine on the bevy discord for
/// suggesting I take a second look at the bevy post-processing example
///
/// If no messages appear on stdout, set to help debug:
///     RUST_LOG="info,wgpu_core=warn,wgpu_hal=warn"
///
/// Source: https://gist.github.com/dmlary/3822b5cda70e562a2226b3372c584ed8
use bevy::{
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::query::QueryItem,
    log::LogPlugin,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            BlendState, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
            MultisampleState, Operations, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, TextureFormat,
        },
        texture::BevyDefault,
        view::ViewTarget,
        RenderApp,
    },
};
use std::env;

fn main() {
    let mut app = App::new();
    let mut args = env::args();
    match args.nth(1) {
        Some(arg) => {
            if arg == "dump" {
                // disable LogPlugin so that you can pipe the output directly into `dot -Tsvg`
                app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
                app.add_plugins(ShaderToyPlugin);
                panic!("No debug dump");
                // bevy_mod_debugdump::print_render_graph(&mut app);
            }
        }
        _ => {
            // enable hot-loading so our shader gets reloaded when it changes
            app.add_plugins((DefaultPlugins, ShaderToyPlugin))
                .add_systems(Startup, setup)
                .run();
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add a cube so it's really clear when the shader doesn't run
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(Cuboid::default())),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(StandardMaterial::default()),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

struct ShaderToyPlugin;

impl Plugin for ShaderToyPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("RenderApp should already exist in App");

        // add our post-processing render node to the render graph
        // place it between tonemapping & the end of post-processing shaders
        render_app
            .add_render_graph_node::<ViewNodeRunner<ShaderToyRenderNode>>(
                Core3d,
                ShaderToyRenderLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    ShaderToyRenderLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app
            .get_sub_app_mut(RenderApp)
            .expect("RenderApp should already exist in App");
        render_app.init_resource::<ShaderToyPipeline>();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ShaderToyRenderLabel;

#[derive(Debug, Default)]
struct ShaderToyRenderNode;

impl ViewNode for ShaderToyRenderNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        (_camera, view_target): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let shader_toy_pipeline = world.resource::<ShaderToyPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache
            .get_render_pipeline(shader_toy_pipeline.pipeline_id)
            .expect("ShaderToyPipeline should be present in the PipelineCache");

        // create a render pass.  Note that we don't want to inherit the
        // color_attachments because then the pipeline Multisample must match
        // whatever msaa was set to.
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("shader_toy_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: view_target.main_texture_view(),
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            ..default()
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.draw(0..4, 0..1);
        Ok(())
    }
}

#[derive(Debug, Resource)]
struct ShaderToyPipeline {
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for ShaderToyPipeline {
    fn from_world(world: &mut World) -> Self {
        let shader = world.resource::<AssetServer>().load("shader_toy.wgsl");

        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("shader_toy_pipeline".into()),
            layout: vec![],
            push_constant_ranges: Vec::new(),
            vertex: bevy::render::render_resource::VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            // default does not work here as we're using TriangleStrip
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: bevy::render::render_resource::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        Self { pipeline_id }
    }
}
