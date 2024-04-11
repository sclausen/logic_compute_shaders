use bevy::log;
use bevy::render::render_resource::{PipelineCache, ShaderType};
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::particle_system::ParticleSystemRender;
use crate::particle_update::ParticleUpdatePipelineConfig;

#[derive(Debug, Copy, Clone, ShaderType, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct Entry {
    original_index: u32,
    hash: u32,
    key: u32,
}

pub fn sort_spatial_hash_grid(
    num_entries: u32,
    queue: &RenderQueue,
    device: &RenderDevice,
    particle_system_render: &ParticleSystemRender,
    context: &ParticleUpdatePipelineConfig,
    pipeline_cache: &PipelineCache,
) {
    log::debug!("Beginning GPU compute for {:?} particles.", num_entries);

    let num_stages = (num_entries.next_power_of_two() as f32).log2() as u32;
    log::debug!("Number of stages: {}", num_stages);
    let sort_pipeline = pipeline_cache
        .get_compute_pipeline(context.sort_pipeline)
        .unwrap();

    let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Indirect Dispatch Buffer"),
        size: std::mem::size_of::<wgpu::util::DispatchIndirectArgs>() as u64,
        usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    for stage_index in 0..num_stages {
        for step_index in 0..=stage_index {
            let mut command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let group_width = 1 << (stage_index - step_index);
            let group_height = 2 * group_width - 1;

            let dispatch_args = wgpu::util::DispatchIndirectArgs {
                x: ((num_entries.next_power_of_two() / 2).max(1) + 255) / 256,
                y: 1,
                z: 1,
            };

            queue.write_buffer(&indirect_buffer, 0, &dispatch_args.as_bytes());

            queue.write_buffer(
                &particle_system_render.group_width_buffer.as_ref().unwrap(),
                0,
                bytemuck::bytes_of(&group_width),
            );
            queue.write_buffer(
                &particle_system_render.group_height_buffer.as_ref().unwrap(),
                0,
                bytemuck::bytes_of(&group_height),
            );
            queue.write_buffer(
                &particle_system_render.step_index_buffer.as_ref().unwrap(),
                0,
                bytemuck::bytes_of(&step_index),
            );

            {
                let mut compute_pass =
                    command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Bitonic Sort Indices Pass"),
                        timestamp_writes: None,
                    });
                compute_pass.set_pipeline(&sort_pipeline);
                compute_pass.set_bind_group(
                    0,
                    &particle_system_render.sort_bind_group.as_ref().unwrap(),
                    &[],
                );
                compute_pass.dispatch_workgroups_indirect(&indirect_buffer, 0);
            }
            queue.submit(Some(command_encoder.finish()));
        }
    }

    let calculate_offsets_pipeline = pipeline_cache
        .get_compute_pipeline(context.calculate_offsets_pipeline)
        .unwrap();

    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let dispatch_args_for_offsets = wgpu::util::DispatchIndirectArgs {
            x: (num_entries + 255) / 256, // Adjust for your compute shader's workgroup size
            y: 1,
            z: 1,
        };
        queue.write_buffer(&indirect_buffer, 0, &dispatch_args_for_offsets.as_bytes());

        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Bitonic Sort Offsets Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&calculate_offsets_pipeline);
        compute_pass.set_bind_group(
            0,
            &particle_system_render.sort_bind_group.as_ref().unwrap(),
            &[],
        );
        //compute_pass.dispatch_workgroups(num_entries, 1, 1);
        compute_pass.dispatch_workgroups_indirect(&indirect_buffer, 0);
    }
    queue.submit(Some(command_encoder.finish()));
}
