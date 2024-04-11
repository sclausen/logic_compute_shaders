use crate::particle::Particle;
use crate::particle_config::ParticleConfig;
use crate::particle_render::{create_render_bind_group, ParticleRenderPipelineConfig};
use crate::particle_update::{
    create_sort_bind_group, create_update_bind_group, ParticleUpdatePipelineConfig,
};
use crate::sort_spatial_hash_grid::Entry;
use crate::system_runner::CommandRunOnce;
use bevy::log;
use bevy::render::extract_resource::ExtractResource;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::texture::ImageSampler;
use bevy::render::Extract;
use bevy::sprite::{Material2d, MaterialMesh2dBundle};
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

pub fn recreate_particles(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut event_writer: EventWriter<RecreateParticles>,
    mut particle_config: ResMut<ParticleConfig>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        *particle_config = ParticleConfig::default();
        event_writer.send(RecreateParticles);
    }
}

pub fn recreate_texture(mut commands: Commands, mut event_reader: EventReader<RecreateParticles>) {
    for _ in event_reader.read() {
        log::debug!("Recreating particle system");
        commands.run_once(setup);
    }
}

pub fn generate_particles(particle_config: &ParticleConfig) -> Vec<Particle> {
    let mut rng = rand::thread_rng();
    (0..particle_config.n)
        .map(|_| {
            let velocity = Vec2::new(0.0, 0.0);
            let position = Vec2::new(
                rng.gen_range(0.0..particle_config.world_width as f32),
                rng.gen_range(0.0..particle_config.world_height as f32),
            );
            let particle_type = rng.gen_range(0..particle_config.variants as u32);

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
            let particles = generate_particles(&particle_config);
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

fn create_texture(images: &mut Assets<Image>, particle_config: &ParticleConfig) -> Handle<Image> {
    let mut image = Image::new_fill(
        Extent3d {
            width: particle_config.world_width,
            height: particle_config.world_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    image.sampler = ImageSampler::nearest();
    images.add(image)
}

pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut image_texture: ResMut<ParticleTexture>,
    mut materials: ResMut<Assets<GrayscaleMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    particle_config: Res<ParticleConfig>,
    query: Query<Entity, With<ParticleSystem>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
        if let Some(handle) = image_texture.0.as_ref() {
            images.remove(handle.clone());
        }
    }

    image_texture.0 = Some(create_texture(&mut images, &particle_config));

    let image = image_texture.0.as_ref().unwrap().clone();
    let width = particle_config.world_width as f32;
    let height = particle_config.world_height as f32;

    let offsets = [
        (-width, height),
        (0.0, height),
        (width, height),
        (-width, 0.0),
        (0.0, 0.0),
        (width, 0.0),
        (-width, -height),
        (0.0, -height),
        (width, -height),
    ];

    commands
        .spawn(SpatialBundle::default())
        .with_children(|parent| {
            for (i, (dx, dy)) in offsets.iter().enumerate() {
                log::debug!(
                    "Creating particle sprite at ({}, {}) with dimensions ({},{}) ",
                    dx,
                    dy,
                    width,
                    height,
                );
                if *dx == 0.0 && *dy == 0.0 {
                    parent.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(width, height)),
                                ..default()
                            },
                            transform: Transform::from_translation(Vec3::new(*dx, *dy, 0.0)),
                            texture: image.clone(),
                            ..default()
                        },
                        Name::new(format!("Particle Image {}", i)),
                    ));
                } else {
                    parent.spawn((
                        MaterialMesh2dBundle {
                            mesh: meshes.add(Rectangle::new(width, height)).into(),
                            transform: Transform::from_translation(Vec3::new(*dx, *dy, 0.0)),
                            material: materials.add(GrayscaleMaterial {
                                texture: Some(image.clone()),
                                is_grayscale: particle_config.is_grayscale as u32,
                            }),
                            ..default()
                        },
                        Name::new(format!("Particle Image {}", i)),
                    ));
                };
            }
        })
        .insert(ParticleSystem {
            rendered_texture: image,
        });
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GrayscaleMaterial {
    #[texture(1)]
    #[sampler(2)]
    texture: Option<Handle<Image>>,
    #[uniform(3)]
    is_grayscale: u32,
}

impl Material2d for GrayscaleMaterial {
    fn fragment_shader() -> ShaderRef {
        "grayscale.wgsl".into()
    }
}

pub fn update_material(
    particle_config: Res<ParticleConfig>,
    material_handle: Query<&Handle<GrayscaleMaterial>>,
    mut materials: ResMut<Assets<GrayscaleMaterial>>,
) {
    for handle in material_handle.iter() {
        let mat = materials.get_mut(handle).unwrap();
        mat.is_grayscale = particle_config.is_grayscale as u32;
    }
}

#[derive(Resource, Default)]
pub struct ParticleTexture(Option<Handle<Image>>);

#[derive(Component)]
pub struct ParticleImage;
