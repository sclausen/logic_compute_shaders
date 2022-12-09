use std::{borrow::Cow, ops::Deref};

use crate::compute_utils::read_buffer;
use crate::particle_render::{ParticleRenderPipeline, RenderParticlesNode};
use crate::particle_update::{ParticleUpdatePipeline, UpdateParticlesNode};
use crate::{Particle, ParticleSystem, PARTICLE_COUNT};
use bevy::{
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
    utils::HashMap,
};
use bevy_inspector_egui::WorldInspectorPlugin;
use wgpu::Maintain;

pub struct ParticlePlugin;

// Must maintain all our own data because render world flushes between frames :,(
#[derive(Resource, Default)]
pub struct ParticleSystemRender {
    pub update_bind_group: HashMap<Entity, BindGroup>,
    pub render_bind_group: HashMap<Entity, BindGroup>,
    pub particle_buffers: HashMap<Entity, Buffer>,
}

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<ParticleSystem>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ParticleUpdatePipeline>()
            .init_resource::<ParticleSystemRender>()
            .init_resource::<ParticleRenderPipeline>()
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let update_node = UpdateParticlesNode::new(&mut render_app.world);
        let render_node = RenderParticlesNode::new(&mut render_app.world);
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("update_particles", update_node);
        render_graph.add_node("render_particles", render_node);

        render_graph
            .add_node_edge("update_particles", "render_particles")
            .unwrap();
        render_graph
            .add_node_edge(
                "render_particles",
                bevy::render::main_graph::node::CAMERA_DRIVER,
            )
            .unwrap();
    }
}

fn queue_bind_group(
    render_device: Res<RenderDevice>,
    //render_queue: Res<RenderQueue>,
    render_pipeline: Res<ParticleRenderPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    mut particle_systems_render: ResMut<ParticleSystemRender>,
    update_pipeline: Res<ParticleUpdatePipeline>,
    //Getting mutable queries in the render world is an antipattern?
    particle_systems: Query<(Entity, &ParticleSystem)>,
) {
    // Everything here is done lazily and should only happen on the first call here.
    for (entity, system) in &particle_systems {
        let view = &gpu_images[&system.image];

        if !particle_systems_render
            .particle_buffers
            .contains_key(&entity)
        {
            let particle = [Particle::default(); PARTICLE_COUNT as usize];
            //ugh
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::StorageBuffer::new(&mut byte_buffer);
            buffer.write(&particle).unwrap();
            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                contents: buffer.into_inner(),
            });
            particle_systems_render
                .particle_buffers
                .insert(entity, storage);
        }

        /*
        read_buffer(
            &particle_systems_render.particle_buffers[&entity],
            &render_device,
            &render_queue,
        );
        */

        if !particle_systems_render
            .update_bind_group
            .contains_key(&entity)
        {
            let update_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &update_pipeline.bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(
                        particle_systems_render.particle_buffers[&entity]
                            .as_entire_buffer_binding(),
                    ),
                }],
            });
            particle_systems_render
                .update_bind_group
                .insert(entity, update_group);
        }

        if !particle_systems_render
            .render_bind_group
            .contains_key(&entity)
        {
            let render_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &render_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(
                            particle_systems_render.particle_buffers[&entity]
                                .as_entire_buffer_binding(),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&view.texture_view),
                    },
                ],
            });
            particle_systems_render
                .render_bind_group
                .insert(entity, render_group);
        }
    }
}

impl ExtractComponent for ParticleSystem {
    type Query = &'static ParticleSystem;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Self {
        // XXX this clone might be expensive, bindgroups are made with an arc, should always be in render world anyway
        item.clone()
    }
}