use std::error::Error;
use wgpu::CompositeAlphaMode;
use wgpu_glyph::{ab_glyph, GlyphBrushBuilder, Region, Section, Text};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new()?;

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let surface = instance.create_surface(&window)?;

    // Initialize GPU
    let (device, queue) = futures::executor::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Request device")
    });

    // Create staging belt
    let mut staging_belt = wgpu::util::StagingBelt::new(1024);

    // Prepare swap chain
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let mut size = window.inner_size();

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: render_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    // Prepare glyph_brush
    let inconsolata = ab_glyph::FontArc::try_from_slice(include_bytes!(
        "Inconsolata-Regular.ttf"
    ))?;

    let mut glyph_brush = GlyphBrushBuilder::using_font(inconsolata)
        .build(&device, render_format);

    // Render loop
    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => elwt.exit(),
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::Resized(new_size),
                    ..
                } => {
                    size = new_size;

                    surface.configure(
                        &device,
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: render_format,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::AutoVsync,
                            alpha_mode: CompositeAlphaMode::Auto,
                            view_formats: vec![],
                            desired_maximum_frame_latency: 2,
                        },
                    );
                }
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::RedrawRequested,
                    ..
                } => {
                    // Get a command encoder for the current frame
                    let mut encoder = device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Redraw"),
                        },
                    );

                    // Get the next frame
                    let frame =
                        surface.get_current_texture().expect("Get next frame");
                    let view = &frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // Clear frame
                    {
                        let _ = encoder.begin_render_pass(
                            &wgpu::RenderPassDescriptor {
                                label: Some("Render pass"),
                                color_attachments: &[Some(
                                    wgpu::RenderPassColorAttachment {
                                        view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(
                                                wgpu::Color {
                                                    r: 0.4,
                                                    g: 0.4,
                                                    b: 0.4,
                                                    a: 1.0,
                                                },
                                            ),
                                            store: wgpu::StoreOp::Store,
                                        },
                                        depth_slice: None,
                                    },
                                )],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            },
                        );
                    }

                    glyph_brush.queue(Section {
                        screen_position: (30.0, 30.0),
                        bounds: (size.width as f32, size.height as f32),
                        text: vec![Text::new("Hello wgpu_glyph!")
                            .with_color([0.0, 0.0, 0.0, 1.0])
                            .with_scale(40.0)],
                        ..Section::default()
                    });

                    // Draw the text!
                    glyph_brush
                        .draw_queued(
                            &device,
                            &mut staging_belt,
                            &mut encoder,
                            view,
                            size.width,
                            size.height,
                        )
                        .expect("Draw queued");

                    glyph_brush.queue(Section {
                        screen_position: (30.0, 90.0),
                        bounds: (size.width as f32, size.height as f32),
                        text: vec![Text::new("Hello wgpu_glyph!")
                            .with_color([1.0, 1.0, 1.0, 1.0])
                            .with_scale(40.0)],
                        ..Section::default()
                    });

                    // Draw the text!
                    glyph_brush
                        .draw_queued_with_transform_and_scissoring(
                            &device,
                            &mut staging_belt,
                            &mut encoder,
                            view,
                            wgpu_glyph::orthographic_projection(
                                size.width,
                                size.height,
                            ),
                            Region {
                                x: 40,
                                y: 105,
                                width: 200,
                                height: 15,
                            },
                        )
                        .expect("Draw queued");

                    // Submit the work!
                    staging_belt.finish();
                    queue.submit(Some(encoder.finish()));
                    frame.present();
                    // Recall unused staging buffers
                    staging_belt.recall();
                }
                _ => {}
            }
        })
        .map_err(Into::into)
}
