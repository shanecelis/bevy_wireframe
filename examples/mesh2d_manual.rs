//! This example shows how to manually render 2d items using "mid level render apis" with a custom
//! pipeline for 2d meshes.
//! It doesn't use the [`Material2d`] abstraction, but changes the vertex buffer to include vertex color.
//! Check out the "mesh2d" example for simpler / higher level 2d meshes.
//!
//! [`Material2d`]: bevy::sprite::Material2d

use bevy::{
    color::palettes::{basic, css},
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    sprite::{Mesh2dHandle, MaterialMesh2dBundle},
};

use std::f32::consts::PI;

use bevy_wireframe::wireframe2d::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [800., 400.].into(),
                title: "Wireframe 2d".into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins((WireframeMesh2dPlugin,))
        .add_systems(Startup, star)
        .run();
}

fn star(
    mut commands: Commands,
    // We will add a new Mesh for the star being created
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Let's define the mesh for the object we want to draw: a nice star.
    // We will specify here what kind of topology is used to define the mesh,
    // that is, how triangles are built from the vertices. We will use a
    // triangle list, meaning that each vertex of the triangle has to be
    // specified. We set `RenderAssetUsages::RENDER_WORLD`, meaning this mesh
    // will not be accessible in future frames from the `meshes` resource, in
    // order to save on memory once it has been uploaded to the GPU.
    let mut star = Mesh::new(
        PrimitiveTopology::TriangleList,
        // FIXME: Main world is required in order to allow PosBuffer to process
        // the mesh too.
        RenderAssetUsages::RENDER_WORLD, // | RenderAssetUsages::MAIN_WORLD,
    );

    // Vertices need to have a position attribute. We will use the following
    // vertices (I hope you can spot the star in the schema).
    //
    //        1
    //
    //     10   2
    // 9      0      3
    //     8     4
    //        6
    //   7        5
    //
    // These vertices are specified in 3D space.
    let mut v_pos = vec![[0.0, 0.0, 0.0]];
    for i in 0..10 {
        // The angle between each vertex is 1/10 of a full rotation.
        let a = i as f32 * PI / 5.0;
        // The radius of inner vertices (even indices) is 100. For outer vertices (odd indices) it's 200.
        let r = (1 - i % 2) as f32 * 100.0 + 100.0;
        // Add the vertex position.
        v_pos.push([r * a.sin(), r * a.cos(), 0.0]);
    }

    // Set the position attribute
    star.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

    // Now, we specify the indices of the vertex that are going to compose the
    // triangles in our star. Vertices in triangles have to be specified in CCW
    // winding (that will be the front face, colored). Since we are using
    // triangle list, we will specify each triangle as 3 vertices
    //   First triangle: 0, 2, 1
    //   Second triangle: 0, 3, 2
    //   Third triangle: 0, 4, 3
    //   etc
    //   Last triangle: 0, 1, 10
    let mut indices = vec![0, 1, 10];
    for i in 2..=10 {
        indices.extend_from_slice(&[0, i, i - 1]);
    }
    star.insert_indices(Indices::U32(indices));
    star.duplicate_vertices();

    // The `Handle<Mesh>` needs to be wrapped in a `Mesh2dHandle` to use 2d
    // rendering instead of 3d.
    // let handle = Mesh2dHandle(meshes.add(star));

    commands.spawn((MaterialMesh2dBundle {
        mesh: meshes.add(star).into(), //Rectangle::new(100.0, 100.0)).into(),
        // material: materials.add(Color::BLACK.with_alpha(0.5)),
        // material: materials.add(Color::from(css::LEMON_CHIFFON)),
        // material: materials.add(Color::WHITE),
        material: materials.add(Color::BLACK),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    }, WireframeMesh2d));
    // commands.spawn((
    //     WireframeMesh2d,
    //     material: materials.add(Color::BLACK),
    //     handle.clone(),
    //     SpatialBundle::INHERITED_IDENTITY,
    // ));
    let shape = Circle { radius: 50.0 };
    let mut circle: Mesh = shape.into();
    circle.duplicate_vertices();
    // circle.asset_usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    let handle = Mesh2dHandle(meshes.add(circle.clone()));

    commands.spawn((
        WireframeMesh2d,
        handle,
        SpatialBundle::from_transform(Transform::from_xyz(-300.0, -100.0, 2.0)),
    ));

    // Spawn the camera
    commands.spawn(Camera2dBundle::default());
}
