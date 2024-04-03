use crate::particle::Particle;
use crate::particle_config::ParticleConfig;
use crate::particle_render::{create_render_bind_group, ParticleRenderPipelineConfig};
use crate::particle_update::{
    create_sort_bind_group, create_update_bind_group, ParticleUpdatePipelineConfig,
};
use crate::sort_spatial_hash_grid::Entry;
use crate::{HEIGHT, WIDTH};
use bevy::render::extract_resource::ExtractResource;
use bevy::render::renderer::RenderQueue;
use bevy::render::Extract;
use bevy::{
    prelude::*,
    render::{
        extract_component::ExtractComponent, render_asset::RenderAssets, render_resource::*,
        renderer::RenderDevice,
    },
};
use rand::Rng;

#[derive(ExtractComponent, Component, Default, Clone)]
pub struct ParticleSystem {
    pub rendered_texture: Handle<Image>,
}

// Must maintain all our own data because render world flushes between frames :,(
#[derive(Resource, Default, Clone, ExtractResource)]
pub struct ParticleSystemRender {
    pub update_bind_group: Option<BindGroup>,
    pub render_bind_group: Option<BindGroup>,
    pub sort_bind_group: Option<BindGroup>,
    pub particle_buffer: Option<Buffer>,
    pub particle_config_buffer: Option<Buffer>,
    pub attraction_matrix_buffer: Option<Buffer>,
    pub delta_time_buffer: Option<Buffer>,
    pub spatial_indices_buffer: Option<Buffer>,
    pub spatial_offsets_buffer: Option<Buffer>,
    pub num_entries_buffer: Option<Buffer>,
    pub group_width_buffer: Option<Buffer>,
    pub group_height_buffer: Option<Buffer>,
    pub step_index_buffer: Option<Buffer>,
    pub output_offsets_buffer: Option<Buffer>,
    pub output_spatial_indices_buffer: Option<Buffer>,
}

#[derive(Event)]
pub struct RecreateParticles;

#[derive(Event)]
pub struct ExtractedRecreateParticles;

pub fn extract_recreate_particles_event(
    mut event_reader: Extract<EventReader<RecreateParticles>>,
    mut event_writer: EventWriter<ExtractedRecreateParticles>,
) {
    for _ in event_reader.read() {
        event_writer.send(ExtractedRecreateParticles);
    }
}

pub fn generate_particles(n: u32, m: u32) -> Vec<Particle> {
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| {
            //     let velocity = Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));
            let velocity = Vec2::new(0.0, 0.0);
            let position = Vec2::new(
                rng.gen_range(0.0..WIDTH as f32),
                rng.gen_range(0.0..HEIGHT as f32),
            );
            let particle_type = rng.gen_range(0..m as u32);

            Particle {
                velocity,
                position,
                particle_type,
            }
        })
        .collect()
}

pub fn queue_bind_group(
    gpu_images: Res<RenderAssets<Image>>,
    particle_config: Res<ParticleConfig>,
    particle_systems: Query<&ParticleSystem>,
    mut particle_system_render: ResMut<ParticleSystemRender>,
    render_device: Res<RenderDevice>,
    render_pipeline_config: Res<ParticleRenderPipelineConfig>,
    time: Res<Time>,
    update_pipeline_config: Res<ParticleUpdatePipelineConfig>,
    mut event_reader: EventReader<ExtractedRecreateParticles>,
) {
    if let Ok(system) = particle_systems.get_single() {
        let recreate = event_reader.read().next().is_some();

        let (shader_particle_config, attraction_matrix) =
            particle_config.extract_shader_variables();

        let spatial_indices_buffer_size =
            (std::mem::size_of::<Entry>() * particle_config.n).next_power_of_two();
        let offsets_buffer_size = std::mem::size_of::<u32>() * particle_config.n;

        if particle_system_render.particle_buffer.is_none() || recreate {
            debug!(
                "Creating particle buffer with {} particles",
                particle_config.n
            );
            let particles = generate_particles(shader_particle_config.n, shader_particle_config.m);
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::StorageBuffer::new(&mut byte_buffer);
            buffer.write(&particles).unwrap();

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                contents: buffer.into_inner(),
            });

            particle_system_render.particle_buffer = Some(storage);
        }

        if particle_system_render.particle_config_buffer.is_none() || recreate {
            debug!("Creating particle config buffer");
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::UniformBuffer::new(&mut byte_buffer);
            buffer.write(&shader_particle_config).unwrap();

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Particle Config Buffer"),
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                contents: buffer.into_inner(),
            });

            particle_system_render.particle_config_buffer = Some(storage);
        }

        if particle_system_render.attraction_matrix_buffer.is_none() || recreate {
            debug!("Creating attraction matrix buffer");
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::StorageBuffer::new(&mut byte_buffer);
            buffer.write(&attraction_matrix).unwrap();

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Attraction Matrix Buffer"),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC, // tut das COPY_SRC not?
                contents: buffer.into_inner(),
            });

            particle_system_render.attraction_matrix_buffer = Some(storage);
        }

        if particle_system_render.delta_time_buffer.is_none() || recreate {
            debug!("Creating delta time buffer");
            let mut byte_buffer = Vec::new();
            let mut buffer = encase::UniformBuffer::new(&mut byte_buffer);
            buffer.write(&time.delta_seconds()).unwrap();

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Delta Time Buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, // ausprobieren ob uniform nicht reicht.
                contents: buffer.into_inner(),
            });

            particle_system_render.delta_time_buffer = Some(storage);
        }

        // if particle_system_render.output_offsets_buffer.is_none() || recreate {
        //     debug!("Creating output offsets buffer");

        //     let storage = render_device.create_buffer(&BufferDescriptor {
        //         label: Some("Output Offsets Buffer"),
        //         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        //         size: offsets_buffer_size as BufferAddress,
        //         mapped_at_creation: false,
        //     });

        //     particle_system_render.output_offsets_buffer = Some(storage);
        // }

        // if particle_system_render
        //     .output_spatial_indices_buffer
        //     .is_none()
        //     || recreate
        // {
        //     debug!("Creating output spatial indices buffer");

        //     let storage = render_device.create_buffer(&BufferDescriptor {
        //         label: Some("Output Spatial Indices Buffer"),
        //         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        //         size: spatial_indices_buffer_size as BufferAddress,
        //         mapped_at_creation: false,
        //     });

        //     particle_system_render.output_spatial_indices_buffer = Some(storage);
        // }

        if particle_system_render.spatial_indices_buffer.is_none() || recreate {
            debug!("Creating spatial indices buffer");

            let storage = render_device.create_buffer(&BufferDescriptor {
                label: Some("Spatial Indices Buffer"),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                size: spatial_indices_buffer_size as BufferAddress,
                mapped_at_creation: false,
            });

            particle_system_render.spatial_indices_buffer = Some(storage);
        }

        if particle_system_render.spatial_offsets_buffer.is_none() || recreate {
            debug!("Creating spatial offsets buffer");

            let storage = render_device.create_buffer(&BufferDescriptor {
                label: Some("Spatial Offsets Buffer"),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                size: offsets_buffer_size as BufferAddress,
                mapped_at_creation: false,
            });

            particle_system_render.spatial_offsets_buffer = Some(storage);
        }

        if particle_system_render.num_entries_buffer.is_none() || recreate {
            debug!("Creating num entries buffer");

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Num Entries Buffer"),
                //contents: bytemuck::cast_slice(&[0u32; 5]),
                contents: bytemuck::bytes_of(&(particle_config.n as u32)),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

            particle_system_render.num_entries_buffer = Some(storage);
        }

        if particle_system_render.group_width_buffer.is_none() || recreate {
            debug!("Creating group width buffer");

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Group Width Buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[0u32; 1]),
            });

            particle_system_render.group_width_buffer = Some(storage);
        }

        if particle_system_render.group_height_buffer.is_none() || recreate {
            debug!("Creating group height buffer");

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Group Height Buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[0u32; 1]),
            });

            particle_system_render.group_height_buffer = Some(storage);
        }

        if particle_system_render.step_index_buffer.is_none() || recreate {
            debug!("Creating step index buffer");

            let storage = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("Step Index Buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[0u32; 1]),
            });

            particle_system_render.step_index_buffer = Some(storage);
        }

        // read_buffer(
        //     &particle_system_render.particle_buffers[&entity],
        //     &render_device,
        //     &render_queue,
        // );

        if particle_system_render.update_bind_group.is_none() || recreate {
            let update_bind_group = create_update_bind_group(
                &render_device,
                &update_pipeline_config,
                &particle_system_render,
            );
            particle_system_render.update_bind_group = Some(update_bind_group);
        }

        if particle_system_render.sort_bind_group.is_none() || recreate {
            let sort_bind_group = create_sort_bind_group(
                &render_device,
                &update_pipeline_config,
                &particle_system_render,
            );
            particle_system_render.sort_bind_group = Some(sort_bind_group);
        }

        if particle_system_render.render_bind_group.is_none() || recreate {
            if let Some(view) = &gpu_images.get(&system.rendered_texture) {
                let render_bind_group = create_render_bind_group(
                    &render_device,
                    &render_pipeline_config,
                    &particle_system_render,
                    view,
                );

                particle_system_render.render_bind_group = Some(render_bind_group);
            }
        }
    }
}
