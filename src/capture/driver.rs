use std::sync::atomic::Ordering;

use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_render::{
    render_asset::RenderAssets,
    render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::{
        CommandEncoderDescriptor, Maintain, MapMode, TexelCopyBufferInfo, TexelCopyBufferLayout,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
};

use crate::capture::{ReleaseBufferSignal, SendBufferJob, WorkerSendBuffer};

use super::Captures;

/// `RenderGraph` label for `CaptureNode`
#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
pub struct CaptureLabel;

/// `RenderGraph` node
#[derive(Default)]
pub struct CaptureDriver;

// Copies image content from render target to buffer
impl render_graph::Node for CaptureDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let captures = world.get_resource::<Captures>().unwrap();
        let gpu_images = world
            .get_resource::<RenderAssets<bevy_render::texture::GpuImage>>()
            .unwrap();

        let mut encoder = render_context
            .render_device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for capture in captures.iter() {
            if !capture.enabled() {
                continue;
            }

            let src_image = gpu_images.get(&capture.src_image).unwrap();

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            // Choose an available buffer

            // we add 1 to start checking the next buffer
            let current = capture.current.load(Ordering::Acquire) + 1;

            let mut chosen = None;
            for i in 0..capture.buffers.len() {
                let idx = (current + i) % capture.buffers.len();
                let buf = &capture.buffers[idx];
                if !buf.in_use.load(Ordering::Acquire) {
                    chosen = Some((idx, buf.clone()));
                    break;
                }
            }

            let Some((idx, buf)) = chosen else {
                info!("All buffers busy, skipping frame");
                capture.skip.store(true, Ordering::Release);
                continue;
            };
            capture.skip.store(false, Ordering::Release);

            capture.current.store(idx, Ordering::Release);

            buf.in_use.store(true, Ordering::Release);

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &buf.buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZero::<u32>::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                src_image.size,
            );
        }

        let render_queue = world.get_resource::<RenderQueue>().unwrap();
        render_queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

pub fn receive_image_from_buffer(
    mut captures: ResMut<Captures>,
    render_device: Res<RenderDevice>,
    worker: Res<WorkerSendBuffer>,
) {
    for (capture_idx, capture) in captures.0.iter_mut().enumerate() {
        if !capture.enabled() {
            continue;
        }

        let skip = capture.skip.load(Ordering::Acquire);
        if skip {
            // info!("Skipping frame");
            continue;
        }

        let current = capture.current.load(Ordering::Acquire);
        let buf = &capture.buffers[current];

        let slice = buf.buffer.slice(..);

        slice.map_async(MapMode::Read, {
            let buffer = buf.buffer.clone();
            let encoder = capture.encoder.clone();
            let in_use = buf.in_use.clone();
            let worker_tx = worker.tx.clone();
            move |result| match result {
                Ok(_) => {
                    let job = SendBufferJob {
                        buffer,
                        encoder,
                        capture_idx,
                        buffer_idx: current,
                    };
                    if let Err(e) = worker_tx.send(job) {
                        error!("Worker channel closed: {:?}", e);
                    }
                }
                Err(err) => {
                    error!("Failed to map buffer: {err}");
                    in_use.store(false, Ordering::Release);
                }
            }
        });

        render_device.poll(Maintain::Poll);
    }
}

pub fn release_mapped_buffers(
    captures: Res<Captures>,
    release_buffer_signal: Res<ReleaseBufferSignal>,
) {
    while let Ok(signal) = release_buffer_signal.rx.try_recv() {
        let capture = &captures[signal.capture_idx];
        let buf = &capture.buffers[signal.buffer_idx];
        buf.buffer.unmap();
        buf.in_use.store(false, Ordering::Release);
    }
}
