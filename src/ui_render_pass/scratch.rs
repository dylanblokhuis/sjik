// struct DOM {
//     pub root: Node,
// }

// trait Element {
//     fn render(&self) -> ();
// }

// struct Node {
//     pub element: Box<dyn Fn() -> Box<dyn Element>>,
//     children: Vec<Box<>,
// }

// impl Node {
//     fn new<F>(f: F) -> Self
//     where
//         F: 'static + Fn() -> Box<dyn Element>,
//     {
//         Self {
//             element: Box::new(f),
//             children: Vec::new(),
//         }
//     }

//     fn append(&mut self, node: Node) {
//         self.children.push(node);
//     }
// }

// struct Div {
//     children: Vec<Box<dyn Element>>,
// }
// impl Element for Div {
//     fn render(&self) -> () {}
// }

// #[test]
// fn test() {
//     fn component() -> Box<dyn Element> {
//         Box::new(Div {
//             children: Vec::new(),
//         })
//     }

//     let node = Node::new(component);
//     let el = node.element.as_ref()();
//     el.render();
//     for child in node.children {
//         let el = child.element.as_ref()();
//         el.render();
//     }
// }

use std::collections::BTreeMap;
use std::{sync::RwLock};

use super::tailwind::{Tailwind};





use std::sync::Arc;

use taffy::{Taffy};


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(usize);

impl EntityId {
    pub fn child(self, entity_id: EntityId) -> EntityId {
        ENTITIES.write().unwrap().add_child(entity_id, self);
        self
    }
}

pub struct Element {
    tw: Tailwind,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
    on_hover: Option<Arc<dyn Fn() + Send + Sync>>,
}

pub struct Entities {
    pub views: BTreeMap<EntityId, Element>,
    pub parents: BTreeMap<EntityId, EntityId>,
}

impl Entities {
    pub fn add(&mut self, view: Element) -> EntityId {
        let id = EntityId(self.views.len());
        println!("ADD - {} {}", id.0, self.views.len());
        self.views.insert(id, view);
        id
    }

    pub fn add_child(&mut self, child: EntityId, parent: EntityId) {
        println!("ADD CHILD - parent: {}  child: {}", parent.0, child.0);
        self.parents.insert(child, parent);
    }
}

pub static ENTITIES: once_cell::sync::Lazy<RwLock<Entities>> = once_cell::sync::Lazy::new(|| {
    RwLock::new(Entities {
        parents: BTreeMap::new(),
        views: BTreeMap::new(),
    })
});

pub fn div(class: &'static str) -> EntityId {
    let tw = Tailwind::new(&class.to_string());
    let id = ENTITIES.write().unwrap().add(Element {
        tw,
        on_click: None,
        on_hover: None,
    });
    id
}

#[derive(Default)]
pub struct Events {
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
    on_hover: Option<Arc<dyn Fn() + Send + Sync>>,
}
impl Events {
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: 'static + Fn() + Send + Sync,
    {
        self.on_click = Some(Arc::new(f));
        self
    }

    pub fn on_hover<F>(mut self, f: F) -> Self
    where
        F: 'static + Fn() + Send + Sync,
    {
        self.on_hover = Some(Arc::new(f));
        self
    }
}

pub fn btn(class: &'static str, events: Events) -> EntityId {
    let tw = Tailwind::new(&class.to_string());
    let id = ENTITIES.write().unwrap().add(Element {
        tw,
        on_click: events.on_click,
        on_hover: events.on_hover,
    });
    id
}

pub struct Layout {
    pub taffy: Taffy,
    pub couples: Vec<(taffy::node::Node, EntityId, Tailwind)>,
    pub root_node: taffy::node::Node,
}

pub fn generate_layout() -> Layout {
    let mut taffy = Taffy::new();
    let entities = ENTITIES.read().unwrap();

    let mut couples = Vec::<(taffy::node::Node, EntityId, Tailwind)>::new();
    for (entity_id, entity) in entities.views.iter() {
        let taffy_id = taffy.new_leaf(entity.tw.layout_style.clone()).unwrap();
        couples.push((taffy_id, *entity_id, entity.tw.clone()));
    }

    for (child, parent) in entities.parents.iter() {
        let parent = couples.iter().find(|(_, id, _)| *id == *parent).unwrap().0;
        let child = couples.iter().find(|(_, id, _)| *id == *child).unwrap().0;
        taffy.add_child(parent, child).unwrap();
    }

    // find out which parent is the root node
    let mut root_node: Option<taffy::node::Node> = None;
    entities.parents.values().for_each(|parent| {
        if !entities.parents.contains_key(parent) {
            root_node = Some(couples.iter().find(|(_, id, _)| *id == *parent).unwrap().0);
        }
    });
    Layout {
        couples,
        taffy,
        root_node: root_node.unwrap(),
    }
}
