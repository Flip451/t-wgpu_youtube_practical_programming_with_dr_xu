use std::{borrow::Cow, sync::Arc};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

#[derive(Default)]
struct App<'a> {
    window: Option<Arc<Window>>,
    config: Option<wgpu::SurfaceConfiguration>,
    surface: Option<wgpu::Surface<'a>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    render_pipeline: Option<wgpu::RenderPipeline>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // リソース初期化の完了を確実にする
        pollster::block_on(async {
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default().with_title("wgpu:03 triangle"))
                    .unwrap(),
            );

            // ウィンドウの作成
            let size = window.inner_size();

            // wgpuの初期化（インスタンスの作成）
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

            // サーフェイスの作成
            let surface = instance
                .create_surface(window.clone())
                .expect("Failed to create a surface");

            // アダプタの取得
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .await
                .expect("Failed to find an appropriate adapter");

            // デバイスの作成
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .expect("Failed to create device");

            // get_preferred_formatの代わりにget_capabilitiesを使用
            let caps = surface.get_capabilities(&adapter);
            let format = caps.formats[0]; // 利用可能なフォーマットの最初のものを使用

            // サーフェイスの設定
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::default(),
                view_formats: vec![],
            };

            // サーフェイスの設定を適用
            surface.configure(&device, &config);

            // シェーダーモジュールの作成
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: Default::default(),
            });

            // すべてのリソースが初期化されたことを確認
            device.poll(wgpu::Maintain::Wait);

            self.window = Some(window);
            self.config = Some(config);
            self.surface = Some(surface);
            self.device = Some(device);
            self.queue = Some(queue);
            self.render_pipeline = Some(render_pipeline);

            println!("リソースの初期化が完了しました。")
        });
    }

    fn window_event(&mut self, target: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                if let (Some(config), Some(surface), Some(device)) = (
                    self.config.as_mut(),
                    self.surface.as_ref(),
                    self.device.as_ref(),
                ) {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(device, config);
                    device.poll(wgpu::Maintain::Wait);
                }
            }
            WindowEvent::CloseRequested => {
                // GPU操作の完了を待つ
                if let Some(device) = &self.device {
                    device.poll(wgpu::Maintain::Wait);
                }

                // リソースを明示的に順番にドロップ
                self.render_pipeline = None;
                self.queue = None;
                self.device = None;
                self.surface = None;
                self.config = None;
                self.window = None; // ウィンドウも明示的にドロップ

                // 最後にイベントループを終了
                target.exit();
            }
            WindowEvent::RedrawRequested => {
                // すべてのリソースが存在する場合のみ描画を実行
                if let (Some(surface), Some(device), Some(queue), Some(render_pipeline)) = (
                    &self.surface,
                    &self.device,
                    &self.queue,
                    &self.render_pipeline.as_ref(),
                ) {
                    match surface.get_current_texture() {
                        Ok(frame) => {
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: None,
                                });
                            {
                                let mut rpass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: None,
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                                        r: 0.05,
                                                        g: 0.062,
                                                        b: 0.08,
                                                        a: 1.0,
                                                    }),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });
                                rpass.set_pipeline(render_pipeline);
                                rpass.draw(0..3, 0..1);
                            }
                            queue.submit(Some(encoder.finish()));
                            frame.present();
                            device.poll(wgpu::Maintain::Wait);
                        }
                        Err(_) => return,
                    }
                }
            }
            _ => {}
        }
    }
}

// main関数の追加
fn main() {
    // Waylandディスプレイサーバーの使用を無効化し、X11を強制的に使用
    std::env::set_var("WAYLAND_DISPLAY", "");

    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(e) => {
            eprintln!("アプリケーションエラー: {}", e);
            std::process::exit(1); // エラー終了
        }
    };

    event_loop.set_control_flow(ControlFlow::Wait);

    env_logger::init();

    let mut app = App::default();
    match event_loop.run_app(&mut app) {
        Ok(_) => std::process::exit(0), // 正常終了
        Err(e) => {
            eprintln!("アプリケーションエラー: {}", e);
            std::process::exit(1); // エラー終了
        }
    }
}
