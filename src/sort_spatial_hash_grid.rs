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

    // queue.write_buffer(
    //     &particle_system_render
    //         .spatial_indices_buffer
    //         .as_ref()
    //         .unwrap(),
    //     0,
    //     bytemuck::cast_slice(local_entries),
    // );
    // queue.write_buffer(
    //     &particle_system_render
    //         .spatial_offsets_buffer
    //         .as_ref()
    //         .unwrap(),
    //     0,
    //     bytemuck::cast_slice(local_offsets),
    // );
    // log::debug!("Wrote to buffer.");

    // let num_entries = local_entries.len() as u32;
    // log::debug!("Number of entries: {}", num_entries);

    // queue.write_buffer(
    //     &particle_system_render.num_entries_buffer.as_ref().unwrap(),
    //     0,
    //     bytemuck::bytes_of(&num_entries),
    // );

    let num_stages = (num_entries.next_power_of_two() as f32).log2() as u32;
    log::debug!("Number of stages: {}", num_stages);
    let sort_pipeline = pipeline_cache
        .get_compute_pipeline(context.sort_pipeline)
        .unwrap();
    for stage_index in 0..num_stages {
        for step_index in 0..=stage_index {
            let mut command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let group_width = 1 << (stage_index - step_index);
            let group_height = 2 * group_width - 1;

            log::debug!(
                "Dispatching stage_index {} step_index {} with group_width {} and group_height {}.",
                stage_index,
                step_index,
                group_width,
                group_height
            );

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
                compute_pass.dispatch_workgroups(num_entries.next_power_of_two() / 2, 1, 1);
            }
            queue.submit(Some(command_encoder.finish()));
        }
    }

    // copy the data from the output_entries_buffer back to the entries_buffer

    // let mut command_encoder =
    //     device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    // let spatial_indices_buffer = &particle_system_render
    //     .spatial_indices_buffer
    //     .as_ref()
    //     .unwrap();
    // command_encoder.copy_buffer_to_buffer(
    //     spatial_indices_buffer,
    //     0,
    //     &particle_system_render
    //         .output_spatial_indices_buffer
    //         .as_ref()
    //         .unwrap(),
    //     0,
    //     spatial_indices_buffer.size(),
    // );

    // queue.submit(Some(command_encoder.finish()));
    // log::debug!("Submitted commands.");
    // let output_entries_buffer_slice = particle_system_render
    //     .output_spatial_indices_buffer
    //     .as_ref()
    //     .unwrap()
    //     .slice(..);

    // let (entries_sender, entries_receiver) = flume::bounded(1);
    // output_entries_buffer_slice.map_async(wgpu::MapMode::Read, move |r| {
    //     entries_sender.send(r).unwrap()
    // });

    // device.poll(wgpu::Maintain::wait()).panic_on_timeout();
    // log::debug!("Device polled.");
    // entries_receiver.recv_async().await.unwrap().unwrap();
    // log::debug!("Indices result received.");
    // {
    //     let view = output_entries_buffer_slice.get_mapped_range();
    //     local_entries.copy_from_slice(bytemuck::cast_slice(&view));
    // }
    // log::debug!("Indices results written to local indices buffer.");
    // particle_system_render
    //     .output_spatial_indices_buffer
    //     .as_ref()
    //     .unwrap()
    //     .unmap();

    let calculate_offsets_pipeline = pipeline_cache
        .get_compute_pipeline(context.calculate_offsets_pipeline)
        .unwrap();

    // Offsets stuff
    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
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
        compute_pass.dispatch_workgroups(num_entries, 1, 1);
    }
    queue.submit(Some(command_encoder.finish()));

    // let mut command_encoder =
    //     device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    // let particle_offsets_buffer = &particle_system_render
    //     .spatial_offsets_buffer
    //     .as_ref()
    //     .unwrap();

    // command_encoder.copy_buffer_to_buffer(
    //     particle_offsets_buffer,
    //     0,
    //     &particle_system_render
    //         .output_offsets_buffer
    //         .as_ref()
    //         .unwrap(),
    //     0,
    //     particle_offsets_buffer.size(),
    // );

    // queue.submit(Some(command_encoder.finish()));

    // let output_offsets_buffer_slice = particle_system_render
    //     .output_offsets_buffer
    //     .as_ref()
    //     .unwrap()
    //     .slice(..);

    // let (offsets_sender, offsets_receiver) = flume::bounded(1);
    // output_offsets_buffer_slice.map_async(wgpu::MapMode::Read, move |r| {
    //     offsets_sender.send(r).unwrap()
    // });

    // device.poll(wgpu::Maintain::wait()).panic_on_timeout();
    // log::debug!("Device polled.");
    // offsets_receiver.recv_async().await.unwrap().unwrap();
    // log::debug!("Offsets result received.");
    // {
    //     let view = output_offsets_buffer_slice.get_mapped_range();
    //     local_offsets.copy_from_slice(bytemuck::cast_slice(&view));
    // }
    // log::debug!("Results written to local offsets buffer.");
    // particle_system_render
    //     .output_offsets_buffer
    //     .as_ref()
    //     .unwrap()
    //     .unmap();
}
