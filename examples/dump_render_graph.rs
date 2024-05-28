use bevy::log::LogPlugin;
use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    // disable LogPlugin so that you can pipe the output directly into `dot -Tsvg`
    app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
    // bevy_mod_debugdump::print_render_graph(&mut app);
}
