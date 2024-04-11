use crate::particle_config::ParticleConfig;
use crate::particle_render::{
    ParticleRenderPipelineConfig, RenderParticlesNode, RenderParticlesRenderLabel,
};
use crate::particle_system::{
    extract_recreate_particles_event, queue_bind_group, recreate_particles, recreate_texture,
    setup, update_material, ExtractedRecreateParticles, GrayscaleMaterial, ParticleSystem,
    ParticleSystemRender, ParticleTexture, RecreateParticles,
};
use crate::particle_ui::ParticleUiPlugin;
use crate::particle_update::{
    ParticleUpdatePipelineConfig, UpdateParticlesNode, UpdateParticlesRenderLabel,
};
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::{graph, Render, RenderSet};
use bevy::sprite::Material2dPlugin;
use bevy::{
    prelude::*,
    render::{extract_component::ExtractComponentPlugin, render_graph::RenderGraph, RenderApp},
};

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleConfig>();
        app.init_resource::<Events<RecreateParticles>>();
        app.init_resource::<ParticleTexture>();

        app.add_plugins((
            Material2dPlugin::<GrayscaleMaterial>::default(),
            ExtractComponentPlugin::<ParticleSystem>::default(),
            ExtractResourcePlugin::<ParticleConfig>::default(),
            ParticleUiPlugin,
        ));
        app.add_systems(PostStartup, setup);
        app.add_systems(
            Update,
            update_material
                .after(recreate_texture)
                .after(recreate_particles),
        );

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<Events<ExtractedRecreateParticles>>()
            .add_systems(ExtractSchedule, extract_recreate_particles_event)
            .add_systems(Render, queue_bind_group.in_set(RenderSet::Queue));

        let update_node = UpdateParticlesNode::new(&mut render_app.world);
        let render_node = RenderParticlesNode::new(&mut render_app.world);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

        render_graph.add_node(UpdateParticlesRenderLabel, update_node);
        render_graph.add_node(RenderParticlesRenderLabel, render_node);

        render_graph.add_node_edge(UpdateParticlesRenderLabel, RenderParticlesRenderLabel);
        render_graph.add_node_edge(RenderParticlesRenderLabel, graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ParticleUpdatePipelineConfig>()
            .init_resource::<ParticleSystemRender>()
            .init_resource::<ParticleRenderPipelineConfig>();
    }
}
