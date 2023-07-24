use beuk::ctx::RenderContext;

use beuk::memory::TextureHandle;
use dioxus::prelude::{Element, Scope, VirtualDom};
use quadtree_rs::area::AreaBuilder;
use quadtree_rs::Quadtree;
use rustc_hash::FxHashSet;
use shipyard::Component;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use taffy::geometry::Point;
use taffy::prelude::Layout;
use tao::{dpi::PhysicalSize, event_loop::EventLoopProxy};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::renderer::Renderer;
use crate::style::Background;
use crate::EventData;
use crate::{
    events::{BlitzEventHandler, DomEvent},
    focus::{Focus, FocusState},
    layout::TaffyLayout,
    mouse::MouseEffected,
    prevent_default::PreventDefault,
    render::render,
    style::{Border, ForgroundColor},
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
    quadtree: Quadtree<u64, NodeId>,
}

impl DioxusApp {
    /// Create a new window state and spawn a vdom thread.
    pub fn new(
        app: fn(Scope) -> Element,
        render_context: &mut RenderContext,
        proxy: EventLoopProxy<Redraw>,
    ) -> Self {
        let mut rdom = RealDom::new([
            MouseEffected::to_type_erased(),
            TaffyLayout::to_type_erased(),
            ForgroundColor::to_type_erased(),
            Background::to_type_erased(),
            Border::to_type_erased(),
            Focus::to_type_erased(),
            PreventDefault::to_type_erased(),
        ]);

        let focus_state = FocusState::create(&mut rdom);

        let dom = DomManager::spawn(
            rdom,
            PhysicalSize {
                width: render_context.render_swapchain.surface_resolution.width,
                height: render_context.render_swapchain.surface_resolution.height,
            },
            app,
            proxy,
        );

        let event_handler = BlitzEventHandler::new(focus_state);
        let renderer = Renderer::new(render_context);

        Self {
            dom,
            renderer,
            event_handler,
            quadtree: Quadtree::new(20),
        }
    }

    pub fn get_attachment_handle(&self) -> TextureHandle {
        self.renderer.attachment_handle
    }

    pub fn render(&mut self, render_context: &mut RenderContext) {
        self.renderer.shapes.clear();
        self.dom.render(&mut self.renderer);
        self.renderer.render(render_context);
        // After we render, we need to update the quadtree to reflect the new positions of the nodes
        self.update_quadtree();
    }

    // TODO: Once we implement a custom tree for Taffy we can call this when the layout actually changes for each node instead of the diffing approach this currently uses
    fn update_quadtree(&mut self) {
        #[derive(Component)]
        struct QuadtreeId(u64);

        fn add_to_quadtree(
            node_id: NodeId,
            parent_location: Point<f32>,
            taffy: &Taffy,
            rdom: &mut RealDom,
            quadtree: &mut Quadtree<u64, NodeId>,
        ) {
            if let Some(node) = rdom.get(node_id) {
                if let Some((size, location)) = {
                    let layout = node.get::<TaffyLayout>();
                    layout.and_then(|l| {
                        if let Ok(Layout { size, location, .. }) = taffy.layout(l.node.unwrap()) {
                            Some((size, location))
                        } else {
                            None
                        }
                    })
                } {
                    let location = Point {
                        x: location.x + parent_location.x,
                        y: location.y + parent_location.y,
                    };

                    let mut qtree_id = None;
                    let area = AreaBuilder::default()
                        .anchor((location.x as u64, location.y as u64).into())
                        .dimensions((size.width as u64, size.height as u64))
                        .build()
                        .unwrap();
                    match node.get::<QuadtreeId>() {
                        Some(id) => {
                            let id = id.0;
                            if let Some(entry) = quadtree.get(id) {
                                let old_area = entry.area();
                                // If the area has changed, we need to update the quadtree
                                if old_area != area {
                                    quadtree.delete_by_handle(id);
                                    qtree_id = quadtree.insert(area, node_id);
                                }
                            } else {
                                // If the node is not in the quadtree, we need to add it
                                qtree_id = quadtree.insert(area, node_id);
                            }
                        }
                        None => {
                            // If the node is not in the quadtree, we need to add it
                            qtree_id = quadtree.insert(area, node_id);
                        }
                    }
                    // Repeat for all children
                    for child in node.child_ids() {
                        add_to_quadtree(child, location, taffy, rdom, quadtree);
                    }
                    // If the node was added or updated, we need to update the node's quadtree id
                    if let Some(id) = qtree_id {
                        let mut node = rdom.get_mut(node_id).unwrap();
                        node.insert(QuadtreeId(id));
                    }
                }
            }
        }
        let mut rdom = self.dom.rdom();
        let taffy = self.dom.taffy();
        add_to_quadtree(
            rdom.root_id(),
            Point::ZERO,
            &taffy,
            &mut rdom,
            &mut self.quadtree,
        );
    }

    pub fn set_size(&mut self, size: PhysicalSize<u32>) {
        // the window size is zero when minimized which causes the renderer to panic
        if size.width > 0 && size.height > 0 {
            self.dom.set_size(size);
            // self.render_context
            //     .resize_surface(&mut self.surface, size.width, size.height);
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
            self.event_handler
                .register_event(event, rdom, taffy, &size, &self.quadtree);
            evts = self.event_handler.drain_events();
        }
        self.dom.send_events(evts);
    }
}

#[allow(clippy::too_many_arguments)]
async fn spawn_dom(
    rdom: Arc<RwLock<RealDom>>,
    taffy: Arc<Mutex<Taffy>>,
    size: Arc<Mutex<PhysicalSize<u32>>>,
    app: fn(Scope) -> Element,
    proxy: EventLoopProxy<Redraw>,
    mut event_receiver: UnboundedReceiver<DomEvent>,
    mut redraw_receiver: UnboundedReceiver<()>,
    vdom_dirty: Arc<FxDashSet<NodeId>>,
) -> Option<()> {
    let mut renderer = DioxusRenderer::new(app, &rdom);
    let mut last_size;

    // initial render
    {
        let mut rdom = rdom.write().ok()?;
        let root_id = rdom.root_id();
        renderer.update(rdom.get_mut(root_id)?);
        let mut ctx = SendAnyMap::new();
        ctx.insert(taffy.clone());
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
        let root_taffy_node = root_node.get::<TaffyLayout>().unwrap().node.unwrap();

        let mut style = locked_taffy.style(root_taffy_node).unwrap().clone();
        style.size = Size {
            width: Dimension::Points(width),
            height: Dimension::Points(height),
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
                let mut rdom = rdom.write().ok()?;
                renderer.handle_event(rdom.get_mut(element)?, name, data, bubbles);
            }
        }

        let mut rdom = rdom.write().ok()?;
        // render after the event has been handled
        let root_id = rdom.root_id();
        renderer.update(rdom.get_mut(root_id)?);

        let mut ctx = SendAnyMap::new();
        ctx.insert(taffy.clone());

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
            last_size = size;
            let mut taffy = taffy.lock().unwrap();
            let root_node = rdom.get(rdom.root_id()).unwrap();
            let root_node_layout = root_node.get::<TaffyLayout>().unwrap();
            let root_taffy_node = root_node_layout.node.unwrap();
            let mut style = taffy.style(root_taffy_node).unwrap().clone();
            let new_size = Size {
                width: Dimension::Points(width),
                height: Dimension::Points(height),
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

    fn render(&self, renderer: &mut Renderer) {
        render(
            &self.rdom(),
            &self.taffy(),
            renderer,
            *self.size.lock().unwrap(),
        );
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
    hot_reload_rx: tokio::sync::mpsc::UnboundedReceiver<dioxus_hot_reload::HotReloadMsg>,
}

impl DioxusRenderer {
    pub fn new(app: fn(Scope) -> Element, rdom: &Arc<RwLock<RealDom>>) -> Self {
        let mut vdom = VirtualDom::new(app);
        let muts = vdom.rebuild();
        let mut rdom = rdom.write().unwrap();
        let mut dioxus_state = DioxusState::create(&mut rdom);
        dioxus_state.apply_mutations(&mut rdom, muts);
        DioxusRenderer {
            vdom,
            dioxus_state,
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
        return Box::pin(async {
            let hot_reload_wait = self.hot_reload_rx.recv();
            let mut hot_reload_msg = None;
            let wait_for_work = self.vdom.wait_for_work();
            tokio::select! {
                Some(msg) = hot_reload_wait => {
                    // #[cfg(all(feature = "hot-reload", debug_assertions))]
                    // {
                        hot_reload_msg = Some(msg);
                    // }
                    // #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
                    // let () = msg;
                }
                _ = wait_for_work => {}
            }
            // if we have a new template, replace the old one
            if let Some(msg) = hot_reload_msg {
                match msg {
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                        self.vdom.replace_template(template);
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        std::process::exit(0);
                    }
                }
            }
        });

        // Box::pin(self.vdom.wait_for_work())
    }
}
