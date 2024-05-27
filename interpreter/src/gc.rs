//! Non-moving tracing collector for all runtime strings (including symbol names).
//!
//! The intern table is weak: only handles explicitly passed to `collect` are roots.
//! Strings have no outgoing edges, so marking these roots completes the trace.
//! Reused slots receive a fresh generation; foreign-heap and stale handles resolve
//! to `None`, never to a different object. No pointers or unsafe code are used.

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_HEAP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    heap: u64,
    slot: usize,
    generation: u64,
}

/// Cumulative counters plus current occupancy. Bytes count UTF-8 payload only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GcStats {
    pub allocations: usize,
    pub collections: usize,
    pub reclaimed: usize,
    pub reused_slots: usize,
    pub live_objects: usize,
    pub live_bytes: usize,
    pub slots: usize,
    pub interned_strings: usize,
}

#[derive(Debug)]
struct Slot {
    generation: u64,
    string: Option<String>,
    marked: bool,
}

/// A safe, non-moving mark/sweep heap with weak string interning.
///
/// ```
/// use interpreter::gc::Heap;
/// let mut heap = Heap::new();
/// let kept = heap.intern("kept");
/// let discarded = heap.intern("discarded");
/// heap.collect([kept]);
/// assert_eq!(heap.get(kept), Some("kept"));
/// assert_eq!(heap.get(discarded), None);
/// ```
#[derive(Debug)]
pub struct Heap {
    id: u64,
    slots: Vec<Slot>,
    free: Vec<usize>,
    // Hash buckets hold handles, not duplicate owned copies of string contents.
    // Equality within a bucket is checked against the heap, so collisions are safe.
    interned: HashMap<u64, Vec<Handle>>,
    stats: GcStats,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        let id = loop {
            let id = NEXT_HEAP_ID.load(Ordering::Relaxed);
            let next = id.checked_add(1).expect("heap identity space exhausted");
            if NEXT_HEAP_ID
                .compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break id;
            }
        };
        Self {
            id,
            slots: Vec::new(),
            free: Vec::new(),
            interned: HashMap::new(),
            stats: GcStats::default(),
        }
    }

    /// Intern without collecting. The owner must collect with its complete root
    /// set before allocation; the VM does this, including in stress-GC mode.
    pub fn intern(&mut self, text: &str) -> Handle {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some(bucket) = self.interned.get(&hash) {
            if let Some(handle) = bucket
                .iter()
                .find(|handle| self.get(**handle) == Some(text))
            {
                return *handle;
            }
        }
        let slot = if let Some(index) = self.free.pop() {
            self.slots[index].string = Some(text.to_owned());
            self.stats.reused_slots += 1;
            index
        } else {
            self.slots.push(Slot {
                generation: 0,
                string: Some(text.to_owned()),
                marked: false,
            });
            self.slots.len() - 1
        };
        let handle = Handle {
            heap: self.id,
            slot,
            generation: self.slots[slot].generation,
        };
        self.interned.entry(hash).or_default().push(handle);
        self.stats.allocations += 1;
        self.stats.live_objects += 1;
        self.stats.live_bytes += text.len();
        self.stats.slots = self.slots.len();
        self.stats.interned_strings += 1;
        handle
    }

    pub fn get(&self, handle: Handle) -> Option<&str> {
        if handle.heap != self.id {
            return None;
        }
        let slot = self.slots.get(handle.slot)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.string.as_deref()
    }

    /// Mark roots, sweep unmarked payloads, and remove dead weak intern entries.
    /// Stale/foreign roots are ignored; passing duplicate roots is harmless.
    pub fn collect(&mut self, roots: impl IntoIterator<Item = Handle>) {
        self.stats.collections += 1;
        for handle in roots {
            if handle.heap == self.id {
                if let Some(slot) = self.slots.get_mut(handle.slot) {
                    if slot.generation == handle.generation && slot.string.is_some() {
                        slot.marked = true;
                    }
                }
            }
        }
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.marked {
                slot.marked = false;
            } else if let Some(text) = slot.string.take() {
                self.stats.reclaimed += 1;
                self.stats.live_objects -= 1;
                self.stats.live_bytes -= text.len();
                // Never wrap a generation: retire an exhausted slot instead.
                if let Some(generation) = slot.generation.checked_add(1) {
                    slot.generation = generation;
                    self.free.push(index);
                }
            }
        }
        let slots = &self.slots;
        self.interned.retain(|_, bucket| {
            bucket.retain(|handle| {
                let slot = &slots[handle.slot];
                slot.generation == handle.generation && slot.string.is_some()
            });
            !bucket.is_empty()
        });
        self.stats.interned_strings = self.stats.live_objects;
    }

    pub fn stats(&self) -> GcStats {
        self.stats
    }
}
