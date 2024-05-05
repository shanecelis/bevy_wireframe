use bevy::prelude::*;
use bevy::log::LogPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>()); // disable LogPlugin so that you can pipe the output directly into `dot -Tsvg`
    bevy_mod_debugdump::print_schedule_graph(&mut app, Update);
    bevy_mod_debugdump::print_render_graph(&mut app);
}
// fn main() {
//     App::new()
//         .add_plugins(
//          // ...<snip>...
//         .add_systems(StartUp, print_render_graph)
//        // ...<snip>...
//         .run();
// }
// pub fn print_render_graph(mut render_graph: ResMut<RenderGraph>) {
//     let dot = bevy_mod_debugdump::render_graph::render_graph_dot(&render_graph);
//     std::fs::write("render-graph.dot", dot)
//       .expect("Failed to write render-graph.dot");;
//     println!("Render graph written to render-graph.dot");
// }
