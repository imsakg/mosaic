//! Traced object graph for the tree interpreter. IDs never escape a run API.
use crate::ast::Function;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) type Id = usize;
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    String(Rc<str>),
    Object(Id),
    Clock,
}
impl Value {
    pub fn truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }
}
#[derive(Clone, Debug)]
pub(crate) enum Object {
    Environment {
        parent: Option<Id>,
        values: HashMap<String, Value>,
    },
    Function {
        declaration: Function,
        closure: Id,
        initializer: bool,
    },
    Class {
        name: String,
        superclass: Option<Id>,
        methods: HashMap<String, Id>,
    },
    Instance {
        class: Id,
        fields: HashMap<String, Value>,
    },
    Bound {
        function: Id,
        instance: Id,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeapStats {
    pub allocations: usize,
    pub collections: usize,
    pub reclaimed: usize,
    pub live_objects: usize,
    pub slots: usize,
}
#[derive(Default)]
pub(crate) struct Heap {
    slots: Vec<Option<Object>>,
    free: Vec<Id>,
    stats: HeapStats,
}
impl Heap {
    pub fn allocate(&mut self, object: Object) -> Id {
        self.stats.allocations += 1;
        self.stats.live_objects += 1;
        if let Some(id) = self.free.pop() {
            self.slots[id] = Some(object);
            id
        } else {
            self.slots.push(Some(object));
            self.stats.slots = self.slots.len();
            self.slots.len() - 1
        }
    }
    pub fn get(&self, id: Id) -> &Object {
        self.slots[id].as_ref().expect("live interpreter object")
    }
    pub fn get_mut(&mut self, id: Id) -> &mut Object {
        self.slots[id].as_mut().expect("live interpreter object")
    }
    pub fn stats(&self) -> HeapStats {
        self.stats
    }
    pub fn collect(&mut self, roots: impl IntoIterator<Item = Id>) {
        let mut marked = vec![false; self.slots.len()];
        let mut pending = roots.into_iter().collect::<Vec<_>>();
        while let Some(id) = pending.pop() {
            if marked[id] {
                continue;
            }
            marked[id] = true;
            let mut values = Vec::new();
            match self.get(id) {
                Object::Environment { parent, values: vs } => {
                    pending.extend(parent);
                    values.extend(vs.values());
                }
                Object::Function { closure, .. } => pending.push(*closure),
                Object::Class {
                    superclass,
                    methods,
                    ..
                } => {
                    pending.extend(superclass);
                    pending.extend(methods.values());
                }
                Object::Instance { class, fields } => {
                    pending.push(*class);
                    values.extend(fields.values());
                }
                Object::Bound { function, instance } => {
                    pending.push(*function);
                    pending.push(*instance);
                }
            }
            for value in values {
                if let Value::Object(id) = value {
                    pending.push(*id);
                }
            }
        }
        for (id, slot) in self.slots.iter_mut().enumerate() {
            if !marked[id] && slot.take_if(|_| true).is_some() {
                self.free.push(id);
                self.stats.reclaimed += 1;
                self.stats.live_objects -= 1;
            }
        }
        self.stats.collections += 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collector_traces_cycles_and_reuses_unreachable_slots() {
        let mut h = Heap::default();
        let root = h.allocate(Object::Environment {
            parent: None,
            values: HashMap::new(),
        });
        let dead = h.allocate(Object::Environment {
            parent: None,
            values: HashMap::new(),
        });
        if let Object::Environment { values, .. } = h.get_mut(dead) {
            values.insert("cycle".into(), Value::Object(dead));
        }
        h.collect([root]);
        assert_eq!(h.stats().live_objects, 1);
        assert_eq!(h.stats().reclaimed, 1);
        assert_eq!(
            h.allocate(Object::Bound {
                function: root,
                instance: root
            }),
            dead
        );
        h.collect([dead]);
        assert_eq!(h.stats().live_objects, 2);
    }
}
