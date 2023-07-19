use std::collections::BTreeMap;
use std::sync::RwLock;

use super::tailwind::Tailwind;

use std::sync::Arc;

use taffy::Taffy;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(pub taffy::node::Node);

impl EntityId {
    pub fn child(self, entity_id: EntityId) -> EntityId {
        ENTITIES.write().unwrap().add_child(entity_id, self);
        self
    }
}

pub struct Element {
    pub tw: Tailwind,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
    on_hover: Option<Arc<dyn Fn() + Send + Sync>>,
}

pub struct Entities {
    pub views: BTreeMap<EntityId, Element>,
    // pub parents: BTreeMap<EntityId, EntityId>,
    pub taffy: Taffy,
}

impl Entities {
    pub fn add(&mut self, view: Element) -> EntityId {
        let id = EntityId(self.taffy.new_leaf(view.tw.layout_style.clone()).unwrap());
        self.views.insert(id, view);
        id
    }

    pub fn add_child(&mut self, child: EntityId, parent: EntityId) {
        // println!("ADD CHILD - parent: {}  child: {}", parent.0, child.0);
        self.taffy.add_child(parent.0, child.0).unwrap();
    }
}

pub static ENTITIES: once_cell::sync::Lazy<RwLock<Entities>> = once_cell::sync::Lazy::new(|| {
    RwLock::new(Entities {
        views: BTreeMap::new(),
        taffy: Taffy::new(),
    })
});

pub fn div(class: &'static str) -> EntityId {
    let tw = Tailwind::new(&class.to_string());
    let id = ENTITIES.write().unwrap().add(Element {
        tw,
        on_click: None,
        on_hover: None,
    });
    println!("{:?} - {:?}", id, class);

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
