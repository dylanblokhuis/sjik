use dioxus_native_core::prelude::*;
use dioxus_native_core_macro::partial_derive_state;
use once_cell::sync::Lazy;
use rustc_hash::FxHashSet;
use shipyard::Component;
use taffy::Taffy;

use crate::{
    render::{get_abs_pos, get_shape},
    style::Tailwind,
};

/// Find the taffy node that contains the given point
fn find_node_on_point_recursive(
    taffy: &Taffy,
    dom: &RealDom,
    mouse_pos: epaint::Pos2,
    node: taffy::tree::NodeId,
    parent_offset: epaint::Pos2, // added parent_offset argument
) -> Option<NodeId> {
    let layout = taffy.layout(node).unwrap();

    let absolute_location = epaint::Pos2 {
        x: layout.location.x + parent_offset.x,
        y: layout.location.y + parent_offset.y,
    };

    if mouse_pos.x >= absolute_location.x
        && mouse_pos.x <= absolute_location.x + layout.size.width
        && mouse_pos.y >= absolute_location.y
        && mouse_pos.y <= absolute_location.y + layout.size.height
    {
        let entity_id = find_dom_element_recursive(dom.get(dom.root_id()).unwrap(), node).unwrap();
        let entity = dom.get(entity_id).unwrap();
        let is_mouse_effected = entity
            .get::<MouseEffected>()
            .filter(|effected| effected.0)
            .is_some();

        if is_mouse_effected {
            return Some(entity.id());
        }

        let children = taffy.children(node).unwrap();
        for child in children {
            if let Some(found) =
                find_node_on_point_recursive(taffy, dom, mouse_pos, child, absolute_location)
            {
                return Some(found);
            }
        }
    }

    None
}

/// Find the DOM element that contains the given taffy node
fn find_dom_element_recursive(node: NodeRef, taffy_node: taffy::tree::NodeId) -> Option<NodeId> {
    let Some(tw) = node.get::<Tailwind>() else {
        return None;
    };

    if tw.node.unwrap() == taffy_node {
        return Some(node.id());
    }

    for child in node.children().iter() {
        let found = find_dom_element_recursive(*child, taffy_node);
        if found.is_some() {
            return found;
        }
    }

    None
}

pub(crate) fn get_hovered(taffy: &Taffy, dom: &RealDom, mouse_pos: epaint::Pos2) -> Option<NodeId> {
    let root_node = dom.get(dom.root_id()).unwrap();
    let tailwind = root_node.get::<Tailwind>().unwrap();

    find_node_on_point_recursive(
        taffy,
        dom,
        mouse_pos,
        tailwind.node.unwrap(),
        epaint::Pos2::ZERO,
    )
}

pub(crate) fn check_hovered(taffy: &Taffy, node: NodeRef, mouse_pos: epaint::Pos2) -> bool {
    let taffy_node = node.get::<Tailwind>().unwrap().node.unwrap();
    let node_layout = taffy.layout(taffy_node).unwrap();
    get_shape(node_layout, node, get_abs_pos(*node_layout, taffy, node))
        .visual_bounding_rect()
        .contains(epaint::Pos2 {
            x: mouse_pos.x,
            y: mouse_pos.y,
        })
}

#[derive(Debug, Default, PartialEq, Clone, Component)]
pub(crate) struct MouseEffected(bool);

#[partial_derive_state]
impl State for MouseEffected {
    type ChildDependencies = ();
    type ParentDependencies = ();
    type NodeDependencies = ();
    const NODE_MASK: NodeMaskBuilder<'static> = NodeMaskBuilder::new().with_listeners();

    fn update<'a>(
        &mut self,
        node_view: NodeView,
        _: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        _: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        _: &SendAnyMap,
    ) -> bool {
        let new = Self(
            node_view
                .listeners()
                .into_iter()
                .flatten()
                .any(|event| MOUSE_EVENTS.contains(&event)),
        );

        if *self != new {
            *self = new;
            true
        } else {
            false
        }
    }

    fn create<'a>(
        node_view: NodeView<()>,
        node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
        parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
        children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
        context: &SendAnyMap,
    ) -> Self {
        let mut myself = Self::default();
        myself.update(node_view, node, parent, children, context);
        myself
    }
}

static MOUSE_EVENTS: Lazy<FxHashSet<&'static str>> = Lazy::new(|| {
    [
        "hover",
        "mouseleave",
        "mouseenter",
        "click",
        "mouseup",
        "mouseclick",
        "mouseover",
    ]
    .into_iter()
    .collect()
});
