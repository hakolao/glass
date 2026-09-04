//! HDR surface example for `glass` (native only).
//!
//! Ports wgpu's HDR test pattern to the `glass` app framework. The window starts
//! in **SDR** and **Space** toggles HDR on/off. HDR is only entered if the
//! surface actually advertises an HDR color space for some format (HDR10 PQ,
//! then extended-linear scRGB, then encoded extended-range sRGB); otherwise the
//! toggle reports that and stays SDR.
//!
//! Toggling changes the surface *format* (e.g. `Rgba16Float` / `Rgb10a2Unorm`
//! for HDR vs the sRGB SDR format), so the render pipeline is rebuilt on each
//! toggle while the shader, bind group layout, uniform buffer and bind group are
//! reused. `Esc` exits (`exit_on_esc`).
//!
//! The test pattern (see the embedded WGSL): a grayscale luminance staircase, a
//! row of BT.709 primaries/secondaries at 203 nits, and a log luminance gradient.
//! On an SDR output the bright staircase patches clip to the same white; on a
//! working HDR output each is visibly brighter than the last.

use glass::prelude::*;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, Buffer, CommandBuffer, RenderPipeline, ShaderModule,
    SurfaceCapabilities, SurfaceColorSpace, SurfaceColorSpaces, SurfaceConfiguration,
    TextureFormat,
};
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::WindowId,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() -> Result<(), GlassError> {
    Glass::run(GlassConfig::performance(), |context| {
        context.create_window("main", WindowConfig {
            title: "glass HDR — SDR".to_string(),
            width: WIDTH,
            height: HEIGHT,
            exit_on_esc: true,
            ..WindowConfig::default()
        });
        Box::new(HdrApp::default())
    })
}

/// A (format, color space, shader mode) triple that fully determines both how
/// the surface is configured and how the shader encodes the pattern.
#[derive(Clone, Copy)]
struct ModeChoice {
    format: TextureFormat,
    color_space: SurfaceColorSpace,
    shader_mode: u32,
}

#[derive(Default)]
struct HdrApp {
    shader: Option<ShaderModule>,
    bind_group_layout: Option<BindGroupLayout>,
    params_buffer: Option<Buffer>,
    bind_group: Option<BindGroup>,
    pipeline: Option<RenderPipeline>,
    /// The SDR mode captured from the surface as `glass` first configured it,
    /// so toggling HDR off returns to exactly that state.
    sdr: Option<ModeChoice>,
    hdr_on: bool,
}

impl GlassApp for HdrApp {
    fn start(&mut self, _event_loop: &ActiveEventLoop, context: &mut GlassContext) {
        // `glass` already configured the surface in SDR when it created the
        // window, so read that back as our SDR baseline (no reconfigure needed).
        let sdr = {
            let window = context.primary_render_window();
            let caps = window.surface().get_capabilities(context.adapter());
            print_capabilities(context.adapter(), &caps);
            let sc = window.surface_config();
            ModeChoice {
                format: sc.format,
                color_space: sc.color_space,
                shader_mode: 0,
            }
        };

        let device = context.device_arc();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hdr pattern"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hdr.wgsl").into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("params"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let params_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("params"),
            contents: params_of(&sdr).map(u32::to_ne_bytes).as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("params"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
        });
        let pipeline = build_pipeline(&device, &shader, &bind_group_layout, sdr.format);

        {
            let window = context.primary_render_window();
            let info = window.surface().display_hdr_info(context.adapter());
            report_display_hdr_info("initial", &info);
        }
        report("Press SPACE to toggle HDR, ESC to exit.");

        self.shader = Some(shader);
        self.bind_group_layout = Some(bind_group_layout);
        self.params_buffer = Some(params_buffer);
        self.bind_group = Some(bind_group);
        self.pipeline = Some(pipeline);
        self.sdr = Some(sdr);
        self.hdr_on = false;
    }

    fn window_input(
        &mut self,
        context: &mut GlassContext,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: &WindowEvent,
    ) {
        if let WindowEvent::KeyboardInput {
            event: key,
            is_synthetic: false,
            ..
        } = event
        {
            if key.state == ElementState::Pressed
                && !key.repeat
                && key.logical_key == Key::Named(NamedKey::Space)
            {
                self.toggle_hdr(context, window_id);
            }
        }
    }

    fn update(&mut self, context: &mut GlassContext) {
        context
            .primary_render_window_mut()
            .render_default(|data| render(self, data));
    }
}

impl HdrApp {
    fn toggle_hdr(&mut self, context: &mut GlassContext, window_id: WindowId) {
        let want_hdr = !self.hdr_on;

        let caps = {
            let window = context.render_window(window_id).expect("window exists");
            window.surface().get_capabilities(context.adapter())
        };

        let choice = if want_hdr {
            match best_hdr_mode(&caps) {
                Some(choice) => choice,
                None => {
                    report("HDR not available on this surface/display — staying SDR.");
                    return;
                }
            }
        } else {
            self.sdr.expect("sdr baseline set in start")
        };

        // Swap the surface format + color space. Everything from `..base` (size,
        // present mode, usage, alpha) is preserved.
        let new_config = {
            let window = context.render_window(window_id).expect("window exists");
            let base = window.surface_config().clone();
            SurfaceConfiguration {
                format: choice.format,
                color_space: choice.color_space,
                ..base
            }
        };
        if let Err(e) = context.configure_surface(&window_id, &new_config) {
            report(format_args!("surface reconfigure failed: {e}"));
            return;
        }

        // New format => new pipeline; new mode => new shader params.
        let device = context.device_arc();
        context.queue().write_buffer(
            self.params_buffer.as_ref().unwrap(),
            0,
            params_of(&choice).map(u32::to_ne_bytes).as_flattened(),
        );
        self.pipeline = Some(build_pipeline(
            &device,
            self.shader.as_ref().unwrap(),
            self.bind_group_layout.as_ref().unwrap(),
            choice.format,
        ));
        self.hdr_on = want_hdr;

        let window = context.render_window(window_id).expect("window exists");
        window.window().set_title(&title_for(&choice, want_hdr));
        let dynamic_range = if choice.color_space.is_hdr() {
            "HDR"
        } else {
            "SDR"
        };
        report(format_args!(
            "Configured {:?} + {:?} ({dynamic_range})",
            choice.format, choice.color_space
        ));
        let info = window.surface().display_hdr_info(context.adapter());
        report_display_hdr_info(if want_hdr { "hdr on" } else { "hdr off" }, &info);
    }
}

/// `[mode, encode_srgb]` uniform contents. `encode_srgb` is set only for the SDR
/// path on a non-sRGB format, where the shader must apply the sRGB OETF itself.
fn params_of(choice: &ModeChoice) -> [u32; 2] {
    [
        choice.shader_mode,
        u32::from(choice.shader_mode == 0 && !choice.format.is_srgb()),
    ]
}

/// Pick the most capable HDR (format, color space) the surface advertises,
/// preferring HDR10 PQ, then extended-linear scRGB, then encoded extended sRGB.
/// Returns `None` when no HDR color space is offered for any format.
fn best_hdr_mode(caps: &SurfaceCapabilities) -> Option<ModeChoice> {
    use SurfaceColorSpace as Cs;
    use SurfaceColorSpaces as Csf;

    const PREFERENCES: &[(Cs, Csf, u32, &[TextureFormat])] = &[
        (Cs::Bt2100Pq, Csf::BT2100_PQ, 2, &[
            TextureFormat::Rgb10a2Unorm,
            TextureFormat::Rgba16Float,
        ]),
        (Cs::ExtendedSrgbLinear, Csf::EXTENDED_SRGB_LINEAR, 1, &[
            TextureFormat::Rgba16Float,
        ]),
        (Cs::ExtendedSrgb, Csf::EXTENDED_SRGB, 4, &[
            TextureFormat::Rgba16Float,
        ]),
    ];

    for &(color_space, flag, shader_mode, preferred_formats) in PREFERENCES {
        // Preferred formats first, then anything else advertising this space.
        let preferred = preferred_formats
            .iter()
            .copied()
            .filter(|&f| caps.color_spaces(f).contains(flag));
        let any = caps
            .format_capabilities
            .iter()
            .filter(|fc| fc.color_spaces.contains(flag))
            .map(|fc| fc.format);
        if let Some(format) = preferred.chain(any).next() {
            return Some(ModeChoice {
                format,
                color_space,
                shader_mode,
            });
        }
    }
    None
}

fn build_pipeline(
    device: &wgpu::Device,
    shader: &ShaderModule,
    bind_group_layout: &BindGroupLayout,
    format: TextureFormat,
) -> RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("hdr"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("hdr test pattern"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn render(app: &mut HdrApp, render_data: RenderData) -> Option<Vec<CommandBuffer>> {
    let RenderData {
        encoder,
        frame,
        ..
    } = render_data;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hdr pattern"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(app.pipeline.as_ref().unwrap());
        pass.set_bind_group(0, app.bind_group.as_ref().unwrap(), &[]);
        pass.draw(0..3, 0..1);
    }
    None
}

fn title_for(choice: &ModeChoice, hdr_on: bool) -> String {
    format!(
        "glass HDR — {} ({:?} + {:?})",
        if hdr_on { "HDR" } else { "SDR" },
        choice.format,
        choice.color_space
    )
}

fn report(msg: impl std::fmt::Display) {
    println!("{msg}");
}

fn print_capabilities(adapter: &wgpu::Adapter, caps: &SurfaceCapabilities) {
    let info = adapter.get_info();
    report(format_args!("Adapter: {} ({:?})", info.name, info.backend));
    report("Surface formats and color spaces:");
    for fc in &caps.format_capabilities {
        report(format_args!("  {:?}: {:?}", fc.format, fc.color_spaces));
    }
    // Spell out the HDR / wide-gamut spaces beyond the universally-supported SDR
    // ones — this is exactly what must be advertised before HDR can be entered.
    let sdr = SurfaceColorSpaces::SRGB | SurfaceColorSpaces::DISPLAY_P3;
    report("HDR / wide-gamut spaces (beyond SDR sRGB / Display-P3):");
    for fc in &caps.format_capabilities {
        let hdr = fc.color_spaces.difference(sdr);
        report(format_args!(
            "  {:?}: {}",
            fc.format,
            if hdr.is_empty() {
                "none (SDR only)".to_owned()
            } else {
                format!("{hdr:?}")
            }
        ));
    }
}

/// Read-only query of what the panel can show right now. Every field is advisory
/// and platform-dependent (`None` == unknown here, not SDR).
fn report_display_hdr_info(source: &str, info: &wgpu::DisplayHdrInfo) {
    report(format_args!("Display HDR info [{source}] (advisory):"));
    report(format_args!("  luminance:      {:?}", info.luminance));
    report(format_args!("  headroom:       {:?}", info.headroom));
    report(format_args!("  chromaticity:   {:?}", info.chromaticity));
    report(format_args!("  coarse:         {:?}", info.coarse));
    report(format_args!("  bits_per_color: {:?}", info.bits_per_color));
    report(format_args!(
        "  -> tone_map_headroom() = {:?}",
        info.tone_map_headroom()
    ));
}
