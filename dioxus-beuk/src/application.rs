use beuk::ctx::RenderContext;

use beuk::memory::ResourceHandle;
use beuk::texture::Texture;
use dioxus::prelude::{Element, Scope, ScopeId, VirtualDom};
use epaint::text::FontDefinitions;
use epaint::{Fonts, TextureManager};
use rustc_hash::FxHashSet;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use taffy::style::Position;
use tao::{dpi::PhysicalSize, event_loop::EventLoopProxy};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::image::ImageExtractor;
use crate::renderer::Renderer;
use crate::style::Tailwind;
use crate::EventData;
use crate::{
    events::{BlitzEventHandler, DomEvent},
    focus::{Focus, FocusState},
    mouse::MouseEffected,
    prevent_default::PreventDefault,
    render::render,
    Redraw, TaoEvent,
};
use dioxus_native_core::{prelude::*, FxDashSet};
use taffy::{
    prelude::{AvailableSpace, Size},
    style::Dimension,
    Taffy,
};

pub struct DioxusApp {
    dom: DomManager,
    renderer: Renderer,
    event_handler: BlitzEventHandler,
}

#[derive(Clone)]
pub struct RendererState {
    pub fonts: Arc<RwLock<Fonts>>,
    pub tex_manager: Arc<RwLock<TextureManager>>,
}

impl DioxusApp {
    /// Create a new window state and spawn a vdom thread.
    pub fn new(
        app: fn(Scope) -> Element,
        render_context: &RenderContext,
        proxy: EventLoopProxy<Redraw>,
    ) -> Self {
        let mut rdom = RealDom::new([
            MouseEffected::to_type_erased(),
            Tailwind::to_type_erased(),
            ImageExtractor::to_type_erased(),
            Focus::to_type_erased(),
            PreventDefault::to_type_erased(),
        ]);

        let focus_state = FocusState::create(&mut rdom);
        let state = RendererState {
            fonts: Arc::new(RwLock::new(Fonts::new(
                1.0,
                8 * 1024,
                FontDefinitions::default(),
            ))),
            tex_manager: Arc::new(RwLock::new(TextureManager::default())),
        };
        let renderer = Renderer::new(render_context, state.clone());

        let swapchain = render_context.get_swapchain();
        let dom = DomManager::spawn(
            rdom,
            state,
            PhysicalSize {
                width: swapchain.surface_resolution.width,
                height: swapchain.surface_resolution.height,
            },
            app,
            proxy,
        );

        let event_handler = BlitzEventHandler::new(focus_state);

        Self {
            dom,
            renderer,
            event_handler,
        }
    }

    pub fn get_attachment_handle(&self) -> &ResourceHandle<Texture> {
        &self.renderer.attachment_handle
    }

    #[tracing::instrument(name = "DioxusApp::render", skip_all)]
    pub fn render(&mut self, render_context: &RenderContext) {
        self.renderer.shapes.clear();

        self.dom.render(&mut self.renderer);
        self.renderer.render(render_context);
    }

    pub fn set_size(&mut self, size: PhysicalSize<u32>) {
        // the window size is zero when minimized which causes the renderer to panic
        if size.width > 0 && size.height > 0 {
            self.dom.set_size(size);
        }
    }

    pub fn clean(&mut self) -> DirtyNodes {
        self.event_handler.clean().or(self.dom.clean())
    }

    pub fn send_event(&mut self, event: &TaoEvent) {
        let size = self.dom.size();
        let size = Size {
            width: size.width,
            height: size.height,
        };
        let evts;
        {
            let rdom = &mut self.dom.rdom();
            let taffy = &self.dom.taffy();
            self.event_handler.register_event(event, rdom, taffy, &size);
            evts = self.event_handler.drain_events();
        }
        self.dom.send_events(evts);
    }
}

#[allow(clippy::too_many_arguments)]
async fn spawn_dom(
    rdom: Arc<RwLock<RealDom>>,
    state: RendererState,
    taffy: Arc<Mutex<Taffy>>,
    size: Arc<Mutex<PhysicalSize<u32>>>,
    app: fn(Scope) -> Element,
    proxy: EventLoopProxy<Redraw>,
    mut event_receiver: UnboundedReceiver<DomEvent>,
    mut redraw_receiver: UnboundedReceiver<()>,
    vdom_dirty: Arc<FxDashSet<NodeId>>,
) -> Option<()> {
    let dom_context = Rc::new(DomContext {
        window_size: RefCell::new(*size.lock().unwrap()),
    });
    let mut renderer = DioxusRenderer::new(app, &rdom, dom_context, proxy.clone());
    let mut last_size;

    // initial render
    {
        let mut rdom = rdom.write().ok()?;
        let root_id = rdom.root_id();

        renderer.update(rdom.get_mut(root_id)?);
        let mut ctx = SendAnyMap::new();
        ctx.insert(taffy.clone());
        ctx.insert(state.clone());
        // update the state of the real dom
        let (to_rerender, _) = rdom.update_state(ctx);
        let size = size.lock().unwrap();

        let width = size.width as f32;
        let height = size.height as f32;
        let size = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };

        last_size = size;

        let mut locked_taffy = taffy.lock().unwrap();

        // the root node fills the entire area
        let root_node = rdom.get(rdom.root_id()).unwrap();
        let root_taffy_node = root_node.get::<Tailwind>().unwrap().node.unwrap();

        let mut style = locked_taffy.style(root_taffy_node).unwrap().clone();

        style.size = Size {
            width: Dimension::Length(width),
            height: Dimension::Length(height),
        };

        locked_taffy.set_style(root_taffy_node, style).unwrap();
        locked_taffy.compute_layout(root_taffy_node, size).unwrap();
        for k in to_rerender.into_iter() {
            vdom_dirty.insert(k);
        }
        proxy.send_event(Redraw).unwrap();
    }

    loop {
        let wait = renderer.poll_async();
        tokio::select! {
            _ = wait => {},
            _ = redraw_receiver.recv() => {},
            Some(event) = event_receiver.recv() => {
                let DomEvent { name, data, element, bubbles } = event;

                let app_ctx = renderer
                    .vdom
                    .base_scope()
                    .consume_context::<Rc<DomContext>>()
                    .unwrap();
                *app_ctx.window_size.borrow_mut() = *size.lock().unwrap();

                let mut rdom = rdom.write().ok()?;
                renderer.handle_event(rdom.get_mut(element)?, name, data, bubbles);
            }
        }

        let mut rdom = rdom.write().ok()?;
        // render after the event has been handled
        let root_id = rdom.root_id();

        let app_ctx = renderer
            .vdom
            .base_scope()
            .consume_context::<Rc<DomContext>>()
            .unwrap();
        *app_ctx.window_size.borrow_mut() = *size.lock().unwrap();

        renderer.update(rdom.get_mut(root_id)?);

        let mut ctx = SendAnyMap::new();
        ctx.insert(taffy.clone());
        ctx.insert(state.clone());

        // update the real dom
        let (to_rerender, _) = rdom.update_state(ctx);

        let size = size.lock().ok()?;

        let width = size.width as f32;
        let height = size.height as f32;
        let size = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        if !to_rerender.is_empty() || last_size != size {
            renderer.vdom.mark_dirty(ScopeId(0));
            last_size = size;
            let mut taffy = taffy.lock().unwrap();
            let root_node = rdom.get(rdom.root_id()).unwrap();
            let root_node_layout = root_node.get::<Tailwind>().unwrap();
            let root_taffy_node = root_node_layout.node.unwrap();
            let mut style = taffy.style(root_taffy_node).unwrap().clone();
            let new_size = Size {
                width: Dimension::Length(width),
                height: Dimension::Length(height),
            };
            if style.size != new_size {
                style.size = new_size;
                taffy.set_style(root_taffy_node, style).unwrap();
            }
            taffy.compute_layout(root_taffy_node, size).unwrap();
            for k in to_rerender.into_iter() {
                vdom_dirty.insert(k);
            }

            proxy.send_event(Redraw).unwrap();
        }
    }
}

/// A wrapper around the RealDom that manages the lifecycle.
struct DomManager {
    rdom: Arc<RwLock<RealDom>>,
    taffy: Arc<Mutex<Taffy>>,
    size: Arc<Mutex<PhysicalSize<u32>>>,
    /// The node that need to be redrawn.
    dirty: Arc<FxDashSet<NodeId>>,
    force_redraw: bool,
    event_sender: UnboundedSender<DomEvent>,
    redraw_sender: UnboundedSender<()>,
}

impl DomManager {
    fn spawn(
        rdom: RealDom,
        state: RendererState,
        size: PhysicalSize<u32>,
        app: fn(Scope) -> Element,
        proxy: EventLoopProxy<Redraw>,
    ) -> Self {
        let rdom: Arc<RwLock<RealDom>> = Arc::new(RwLock::new(rdom));
        let taffy = Arc::new(Mutex::new(Taffy::new()));
        let size = Arc::new(Mutex::new(size));
        let dirty = Arc::new(FxDashSet::default());

        let (event_sender, event_receiver) = unbounded_channel::<DomEvent>();
        let (redraw_sender, redraw_receiver) = unbounded_channel::<()>();

        let (rdom_clone, size_clone, dirty_clone, taffy_clone) =
            (rdom.clone(), size.clone(), dirty.clone(), taffy.clone());
        // Spawn a thread to run the virtual dom and update the real dom.
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(spawn_dom(
                    rdom_clone,
                    state,
                    taffy_clone,
                    size_clone,
                    app,
                    proxy,
                    event_receiver,
                    redraw_receiver,
                    dirty_clone,
                ));
        });

        Self {
            rdom,
            taffy,
            size,
            dirty,
            event_sender,
            redraw_sender,
            force_redraw: false,
        }
    }

    fn clean(&self) -> DirtyNodes {
        if self.force_redraw {
            DirtyNodes::All
        } else {
            let dirty = self.dirty.iter().map(|k| *k.key()).collect();
            self.dirty.clear();
            DirtyNodes::Some(dirty)
        }
    }

    fn rdom(&self) -> RwLockWriteGuard<RealDom> {
        self.rdom.write().unwrap()
    }

    fn taffy(&self) -> MutexGuard<Taffy> {
        self.taffy.lock().unwrap()
    }

    fn set_size(&mut self, size: PhysicalSize<u32>) {
        *self.size.lock().unwrap() = size;
        self.force_redraw();
    }

    fn size(&self) -> PhysicalSize<u32> {
        *self.size.lock().unwrap()
    }

    fn force_redraw(&mut self) {
        self.force_redraw = true;
        self.redraw_sender.send(()).unwrap();
    }

    #[tracing::instrument(name = "DomManager::render", skip_all)]
    fn render(&self, renderer: &mut Renderer) {
        render(&self.rdom(), &self.taffy(), renderer);
    }

    fn send_events(&self, events: impl IntoIterator<Item = DomEvent>) {
        for evt in events {
            let _ = self.event_sender.send(evt);
        }
    }
}

pub enum DirtyNodes {
    All,
    Some(FxHashSet<NodeId>),
}

impl DirtyNodes {
    pub fn is_empty(&self) -> bool {
        match self {
            DirtyNodes::All => false,
            DirtyNodes::Some(v) => v.is_empty(),
        }
    }

    #[allow(dead_code)]
    pub fn or(self, other: DirtyNodes) -> DirtyNodes {
        match (self, other) {
            (DirtyNodes::All, _) => DirtyNodes::All,
            (_, DirtyNodes::All) => DirtyNodes::All,
            (DirtyNodes::Some(mut v1), DirtyNodes::Some(v2)) => {
                v1.extend(v2);
                DirtyNodes::Some(v1)
            }
        }
    }
}

struct DioxusRenderer {
    vdom: VirtualDom,
    dioxus_state: DioxusState,
    proxy: EventLoopProxy<Redraw>,
    #[cfg(feature = "hot-reload")]
    hot_reload_rx: tokio::sync::mpsc::UnboundedReceiver<dioxus_hot_reload::HotReloadMsg>,
}

#[derive(Clone, Debug, Default)]
pub struct DomContext {
    pub window_size: RefCell<PhysicalSize<u32>>,
}

impl DioxusRenderer {
    pub fn new(
        app: fn(Scope) -> Element,
        rdom: &Arc<RwLock<RealDom>>,
        ctx: Rc<DomContext>,
        proxy: EventLoopProxy<Redraw>,
    ) -> Self {
        let mut vdom = VirtualDom::new(app).with_root_context(ctx);
        let muts = vdom.rebuild();
        let mut rdom = rdom.write().unwrap();
        let mut dioxus_state = DioxusState::create(&mut rdom);
        dioxus_state.apply_mutations(&mut rdom, muts);
        DioxusRenderer {
            vdom,
            dioxus_state,
            proxy,
            #[cfg(feature = "hot-reload")]
            hot_reload_rx: {
                let (hot_reload_tx, hot_reload_rx) =
                    tokio::sync::mpsc::unbounded_channel::<dioxus_hot_reload::HotReloadMsg>();
                dioxus_hot_reload::connect(move |msg| {
                    let _ = hot_reload_tx.send(msg);
                });
                hot_reload_rx
            },
        }
    }

    fn update(&mut self, mut root: NodeMut<()>) {
        let rdom = root.real_dom_mut();

        let muts = self.vdom.render_immediate();
        self.dioxus_state.apply_mutations(rdom, muts);
    }

    fn handle_event(
        &mut self,
        node: NodeMut<()>,
        event: &str,
        value: Arc<EventData>,
        bubbles: bool,
    ) {
        if let Some(id) = node.mounted_id() {
            self.vdom
                .handle_event(event, value.deref().clone().into_any(), id, bubbles);
        }
    }

    fn poll_async(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>> {
        #[cfg(feature = "hot-reload")]
        return Box::pin(async {
            let hot_reload_wait = self.hot_reload_rx.recv();
            let mut hot_reload_msg = None;
            let wait_for_work = self.vdom.wait_for_work();
            tokio::select! {
                Some(msg) = hot_reload_wait => {
                    hot_reload_msg = Some(msg);
                }
                _ = wait_for_work => {}
            }
            // if we have a new template, replace the old one
            if let Some(msg) = hot_reload_msg {
                match msg {
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                        self.vdom.replace_template(template);
                        self.proxy.send_event(Redraw).unwrap();
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        std::process::exit(0);
                    }
                }
            }
        });

        #[cfg(not(feature = "hot-reload"))]
        Box::pin(self.vdom.wait_for_work())
    }
}
