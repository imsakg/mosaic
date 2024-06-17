mod common;
use common::*;
use interpreter::ast::Op;
use interpreter::vm::Vm;

fn stress_vm() -> Vm {
    let mut vm = Vm::with_gc_threshold(1);
    vm.set_gc_stress(true);
    vm
}

#[test]
fn stack_temporaries_survive_collection_while_the_right_operand_allocates() {
    let mut vm = stress_vm();
    let joined = binary(
        binary(text("ab"), Op::Add, text("cd")),
        Op::Add,
        binary(text("ef"), Op::Add, text("gh")),
    );
    assert_eq!(vm.run(&[print(joined)]).unwrap(), ["abcdefgh"]);
    let stats = vm.gc_stats();
    assert!(stats.collections >= 8, "{stats:?}");
    assert_eq!(stats.live_objects, 0);
    assert!(stats.reclaimed >= 7, "{stats:?}");
    assert!(stats.reused_slots > 0);
}

#[test]
fn global_initializers_assignments_and_symbol_names_are_gc_roots() {
    let mut vm = stress_vm();
    vm.run(&[var(
        "global",
        Some(binary(text("per"), Op::Add, text("manent"))),
    )])
    .unwrap();
    for i in 0..100 {
        let new_value = format!("value-{i}");
        assert_eq!(
            vm.run(&[
                expression(binary(text("temporary"), Op::Add, text("garbage"))),
                print(name("global")),
                expression(assign("global", text(&new_value))),
            ])
            .unwrap()
            .len(),
            1
        );
        vm.collect_garbage();
        assert_eq!(vm.run(&[print(name("global"))]).unwrap(), [new_value]);
        assert_eq!(vm.gc_stats().live_objects, 2);
    }
    assert!(vm.gc_stats().reclaimed > 100);
    assert!(vm.gc_stats().reused_slots > 100);
}

#[test]
fn lexical_environment_keys_and_values_survive_collections_in_nested_blocks() {
    let mut vm = stress_vm();
    let program = vec![block(vec![
        var("outer", Some(text("outer-value"))),
        block(vec![
            var("inner", Some(text("inner-value"))),
            expression(binary(text("a"), Op::Add, text("b"))),
            print(name("outer")),
            print(name("inner")),
            expression(assign(
                "outer",
                binary(name("inner"), Op::Add, text("-assigned")),
            )),
        ]),
        print(name("outer")),
    ])];
    assert_eq!(
        vm.run(&program).unwrap(),
        ["outer-value", "inner-value", "inner-value-assigned"]
    );
    assert_eq!(vm.gc_stats().live_objects, 0);
    assert_eq!(vm.gc_stats().interned_strings, 0);
}

#[test]
fn thousands_of_allocations_are_swept_and_slots_reused_during_execution() {
    let mut vm = stress_vm();
    let program = vec![block(vec![
        var("i", Some(num(0.0))),
        var("s", Some(text(""))),
        while_(
            binary(name("i"), Op::Less, num(1_200.0)),
            block(vec![
                var(
                    "temporary",
                    Some(binary(text("trans"), Op::Add, text("ient"))),
                ),
                expression(assign("s", binary(name("s"), Op::Add, text("x")))),
                expression(assign("i", binary(name("i"), Op::Add, num(1.0)))),
            ]),
        ),
        print(name("s")),
    ])];
    assert_eq!(vm.run(&program).unwrap(), ["x".repeat(1_200)]);
    let stats = vm.gc_stats();
    assert_eq!(stats.live_objects, 0);
    assert_eq!(stats.live_bytes, 0);
    assert_eq!(stats.allocations, stats.reclaimed);
    assert!(stats.allocations > 5_000, "{stats:?}");
    assert!(stats.collections > 5_000, "{stats:?}");
    assert!(stats.reused_slots > 5_000, "{stats:?}");
    assert!(stats.slots <= 16, "{stats:?}");
    println!("GC stress stats: {stats:?}");
}

#[test]
fn runtime_failure_discards_local_roots_but_preserves_completed_global_effects() {
    let mut vm = stress_vm();
    vm.run(&[var("kept", Some(text("old")))]).unwrap();
    let error = vm
        .run(&[
            expression(assign("kept", text("new"))),
            block(vec![
                var("local", Some(text("secret"))),
                print(binary(text("left"), Op::Add, name("undefined"))),
            ]),
        ])
        .unwrap_err();
    assert!(error.contains("Undefined variable"));
    assert_eq!(vm.gc_stats().live_objects, 2);
    assert_eq!(vm.run(&[print(name("kept"))]).unwrap(), ["new"]);
    assert!(vm.run(&[print(name("local"))]).is_err());
    assert_eq!(vm.gc_stats().live_objects, 2);
}

#[test]
fn global_name_and_value_can_share_one_interned_heap_object() {
    let mut vm = stress_vm();
    vm.run(&[var("same", Some(text("same")))]).unwrap();
    assert_eq!(vm.gc_stats().live_objects, 1);
    assert_eq!(vm.run(&[print(name("same"))]).unwrap(), ["same"]);
    vm.run(&[expression(assign("same", nil()))]).unwrap();
    assert_eq!(vm.gc_stats().live_objects, 1); // symbol key is still a root
}

#[test]
fn automatic_threshold_collection_is_active_without_stress_mode() {
    let mut vm = Vm::with_gc_threshold(2);
    let program: Vec<_> = (0..500)
        .map(|i| expression(text(&format!("discard-{i}"))))
        .collect();
    assert!(vm.run(&program).unwrap().is_empty());
    let stats = vm.gc_stats();
    assert!(stats.collections > 100, "{stats:?}");
    assert!(stats.slots <= 2, "{stats:?}");
    assert_eq!(stats.allocations, 500);
    assert_eq!(stats.reclaimed, 500);
}

#[test]
fn repeated_runs_do_not_accumulate_stack_or_environment_roots() {
    let mut vm = stress_vm();
    for i in 0..500 {
        let text = format!("iteration-{i}");
        vm.run(&[
            var("retained", Some(common::text(&text))),
            block(vec![var("ephemeral", Some(common::text(&text)))]),
        ])
        .unwrap();
        assert_eq!(vm.run(&[print(name("retained"))]).unwrap(), [text]);
        assert_eq!(vm.gc_stats().live_objects, 2);
    }
    assert!(vm.gc_stats().slots < 10, "{:?}", vm.gc_stats());
}
