use interpreter::gc::Heap;

#[test]
fn roots_survive_and_unreachable_strings_are_swept() {
    let mut heap = Heap::new();
    let live = heap.intern("rooted");
    let dead = heap.intern("garbage");
    heap.collect([live]);
    assert_eq!(heap.get(live), Some("rooted"));
    assert_eq!(heap.get(dead), None);
    let stats = heap.stats();
    assert_eq!(stats.live_objects, 1);
    assert_eq!(stats.live_bytes, "rooted".len());
    assert_eq!(stats.reclaimed, 1);
    assert_eq!(stats.collections, 1);
}

#[test]
fn weak_interning_does_not_keep_objects_alive() {
    let mut heap = Heap::new();
    let first = heap.intern("same");
    assert_eq!(heap.intern("same"), first);
    assert_eq!(heap.stats().allocations, 1);
    assert_eq!(heap.stats().interned_strings, 1);
    heap.collect([]);
    assert_eq!(heap.stats().live_objects, 0);
    assert_eq!(heap.stats().interned_strings, 0);
    let replacement = heap.intern("same");
    assert_ne!(first, replacement);
    assert_eq!(heap.get(first), None);
    assert_eq!(heap.get(replacement), Some("same"));
    assert_eq!(heap.stats().slots, 1);
    assert_eq!(heap.stats().reused_slots, 1);
}

#[test]
fn stale_roots_cannot_keep_a_reused_slot_alive() {
    let mut heap = Heap::new();
    let stale = heap.intern("old");
    heap.collect([]);
    let fresh = heap.intern("new");
    heap.collect([stale]);
    assert_eq!(heap.get(fresh), None);
    assert_eq!(heap.stats().live_objects, 0);
}

#[test]
fn handles_cannot_alias_across_heaps() {
    let mut first = Heap::new();
    let mut second = Heap::new();
    let a = first.intern("first");
    let b = second.intern("second");
    assert_ne!(a, b);
    assert_eq!(second.get(a), None);
    assert_eq!(first.get(b), None);
    second.collect([a]);
    assert_eq!(second.get(b), None);
    assert_eq!(first.get(a), Some("first"));
}

#[test]
fn unicode_and_empty_strings_track_payload_bytes() {
    let mut heap = Heap::new();
    let empty = heap.intern("");
    let unicode = heap.intern("müzik 🎼");
    heap.collect([empty, unicode, unicode]);
    assert_eq!(heap.get(empty), Some(""));
    assert_eq!(heap.get(unicode), Some("müzik 🎼"));
    assert_eq!(heap.stats().live_bytes, "müzik 🎼".len());
    assert_eq!(heap.stats().live_objects, 2);
}

#[test]
fn repeated_sweeps_reuse_storage_without_resurrecting_handles() {
    let mut heap = Heap::new();
    let permanent = heap.intern("permanent");
    let mut stale = Vec::new();
    for i in 0..2_000 {
        let temporary = heap.intern(&format!("temporary-{i}"));
        stale.push(temporary);
        heap.collect([permanent]);
        assert_eq!(heap.get(temporary), None);
        assert_eq!(heap.get(permanent), Some("permanent"));
    }
    assert!(stale.iter().all(|handle| heap.get(*handle).is_none()));
    let stats = heap.stats();
    assert_eq!(stats.live_objects, 1);
    assert_eq!(stats.slots, 2);
    assert_eq!(stats.reclaimed, 2_000);
    assert_eq!(stats.reused_slots, 1_999);
}
