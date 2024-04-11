use bevy::{
    log::LogPlugin,
    prelude::*,
    window::WindowResolution,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub const WORKGROUP_SIZE: u32 = 16;

mod compute_utils;
mod particle;
mod particle_config;
mod particle_plugin;
mod particle_render;
mod particle_system;
mod particle_ui;
mod particle_update;
mod sort_spatial_hash_grid;
mod system_runner;

use particle_plugin::ParticlePlugin;
use particle_system::ParticleSystem;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1024.0, 1024.0),
                    title: "Bevy Particle Simulation".to_string(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(LogPlugin {
                filter: "info,wgpu_core=warn,wgpu_hal=warn,logic_gpu_particles=debug,logic_gpu_particles::sort_spatial_hash_grid=info".into(),
                level: bevy::log::Level::DEBUG,
                update_subscriber: None,
            }), 
    )
  //  .add_plugins(WorldInspectorPlugin::new())
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(ParticlePlugin)
    .insert_resource(Msaa::Off)
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Startup, setup);

    #[cfg(feature = "debug")]
    {
        use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
        app.add_plugins((FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin::default()));
    }

    app.run();
}

fn setup(
    mut commands: Commands,
) {
    commands.spawn(Camera2dBundle::default());
}
