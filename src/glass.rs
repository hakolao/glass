//! The [`Glass`] runner: drives the winit event loop and calls into your [`GlassApp`].

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowId,
};

use crate::{GlassApp, GlassConfig, GlassContext, GlassError};

/// [`Glass`] is an application that exposes an easy to use API to organize your winit applications
/// which render using wgpu. Just impl [`GlassApp`] for your application (of any type) and you
/// are good to go.
pub struct Glass {
    app: Box<dyn GlassApp>,
    context: GlassContext,
    runner_state: RunnerState,
}

// Written by hand rather than derived, because the boxed [`GlassApp`] is opaque: requiring
// `Debug` of it would force the bound on every user app.
impl std::fmt::Debug for Glass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Glass")
            .field("app", &"Box<dyn GlassApp>")
            .field("context", &self.context)
            .field("runner_state", &self.runner_state)
            .finish()
    }
}

impl Glass {
    /// Creates the device context, the event loop and your app, then runs until every window is
    /// closed or [`GlassContext::exit`] is called.
    ///
    /// `app_create_fn` receives the context before the event loop starts, so it is the right place
    /// to call [`GlassContext::create_window`] and to build pipelines.
    ///
    /// # Errors
    ///
    /// Fails if the device context cannot be created, if the event loop cannot be created or run,
    /// or if a window queued through [`GlassContext::create_window`] fails to be created. The last
    /// case ends the event loop and returns the failure here, because deferred creation has no
    /// other way to report it.
    pub fn run(
        config: GlassConfig,
        app_create_fn: impl FnOnce(&mut GlassContext) -> Box<dyn GlassApp>,
    ) -> Result<(), GlassError> {
        let mut context = GlassContext::new(config.clone())?;
        let event_loop = EventLoop::new()?;
        let app = app_create_fn(&mut context);
        let mut glass = Glass {
            app,
            context,
            runner_state: RunnerState {
                run_extra_update_on_resize: config.run_extra_update_on_resize,
                is_surface_auto_resize: config.is_surface_auto_resize,
                ..RunnerState::default()
            },
        };
        event_loop.run_app(&mut glass)?;
        // A window that failed to spawn during the loop ends it; report that instead of `Ok`.
        match glass.runner_state.deferred_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl ApplicationHandler for Glass {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        // Ensure we're poll
        if event_loop.control_flow() != ControlFlow::Poll {
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        let Glass {
            app,
            context,
            ..
        } = self;
        app.before_input(context, event_loop);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.add_new_windows(event_loop) {
            return;
        }
        let Glass {
            app,
            context,
            runner_state,
            ..
        } = self;
        // Initial windows
        if !runner_state.is_init {
            app.start(event_loop, context);
            runner_state.is_init = true;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Glass {
            app,
            context,
            runner_state,
            ..
        } = self;
        app.window_input(context, event_loop, window_id, &event);

        let device = context.device_arc();
        let Some(window) = context.render_window_mut(window_id) else {
            return;
        };
        match event {
            WindowEvent::Resized(physical_size) => {
                if runner_state.is_surface_auto_resize {
                    // On windows, minimized app can have 0,0 size
                    if physical_size.width > 0 && physical_size.height > 0 {
                        let _ = window.configure_surface_with_size(&device, physical_size);
                        if runner_state.run_extra_update_on_resize {
                            context.set_resize_extra_update(true);
                            run_update(event_loop, app, context, runner_state);
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged {
                ..
            } => {
                if runner_state.is_surface_auto_resize {
                    let size = window.window().inner_size();
                    let _ = window.configure_surface_with_size(&device, size);
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && !is_synthetic
                    && window.exit_on_esc()
                    && window.is_focused()
                    && event.state == ElementState::Pressed
                {
                    runner_state.request_window_close = true;
                    runner_state.remove_windows.push(window_id);
                }
            }
            WindowEvent::Focused(has_focus) => {
                window.set_focus(has_focus);
            }
            WindowEvent::CloseRequested => {
                runner_state.request_window_close = true;
                runner_state.remove_windows.push(window_id);
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Glass {
            app,
            context,
            ..
        } = self;
        app.device_input(context, event_loop, device_id, &event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.add_new_windows(event_loop) {
            return;
        }
        let Glass {
            app,
            context,
            runner_state,
            ..
        } = self;
        context.set_resize_extra_update(false);
        run_update(event_loop, app, context, runner_state);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let Glass {
            app,
            context,
            ..
        } = self;
        app.end(context);
    }
}

impl Glass {
    /// Creates the windows queued through [`GlassContext::create_window`].
    ///
    /// Returns `false` when creation failed, in which case the error is stashed for
    /// [`Glass::run`] to return and the event loop is asked to exit.
    fn add_new_windows(&mut self, event_loop: &ActiveEventLoop) -> bool {
        match self.context.create_queued_windows(event_loop) {
            Ok(()) => true,
            Err(e) => {
                log::error!("failed to create a queued window: {e}");
                self.runner_state.deferred_error = Some(e);
                event_loop.exit();
                false
            }
        }
    }
}

fn run_update(
    event_loop: &ActiveEventLoop,
    app: &mut Box<dyn GlassApp>,
    context: &mut GlassContext,
    runner_state: &mut RunnerState,
) {
    if context.should_exit() {
        context.clear_windows();
        event_loop.exit();
        return;
    }
    if runner_state.request_window_close {
        for window in runner_state.remove_windows.iter() {
            context.remove_window(window);
        }
        runner_state.remove_windows.clear();
        runner_state.request_window_close = false;
        // Exit
        if !context.has_windows() {
            context.exit();
            return;
        }
    }
    app.update(context);
}

#[derive(Debug, Default)]
struct RunnerState {
    is_init: bool,
    request_window_close: bool,
    run_extra_update_on_resize: bool,
    is_surface_auto_resize: bool,
    remove_windows: Vec<WindowId>,
    /// Failure from a window queued via [`GlassContext::create_window`], returned by
    /// [`Glass::run`] once the loop has ended.
    deferred_error: Option<GlassError>,
}
