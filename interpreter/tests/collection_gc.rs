use interpreter::{error::Diagnostic, tree::Interpreter};

#[test]
fn closures_and_bound_methods_survive_collection_only_root_paths() {
    let mut interpreter = Interpreter::new();
    interpreter
        .run(
            r#"
        var saved;
        {
            class Cell {
                init(value) { this.value = value; }
                read() { return this.value; }
            }
            var cell = Cell(41);
            saved = map(tuple("callback", tuple(cell.read)));
        }
    "#,
        )
        .unwrap();
    for _ in 0..10 {
        interpreter.run("").unwrap();
        assert_eq!(
            interpreter
                .run(r#"print get(get(saved, "callback"), 0)();"#)
                .unwrap(),
            ["41"]
        );
    }
    interpreter.run("saved = nil;").unwrap();
    interpreter.run("").unwrap();
    assert_eq!(interpreter.heap_stats().live_objects, 1);
}

#[test]
fn object_cycles_through_maps_and_tuples_do_not_leak() {
    let mut interpreter = Interpreter::new();
    for _ in 0..100 {
        interpreter
            .run(
                r#"{
            class Node {}
            var node = Node();
            node.back = map(tuple("self", tuple(node)));
        }"#,
            )
            .unwrap();
        assert_eq!(interpreter.heap_stats().live_objects, 1);
    }
    let stats = interpreter.heap_stats();
    assert_eq!(stats.allocations - stats.reclaimed, 1);
    assert!(stats.slots < 10);
}

#[test]
fn collection_depth_and_direct_item_bounds_leave_the_session_reusable() {
    let mut interpreter = Interpreter::new();
    let error = interpreter
        .run("var t = nil; for (var i=0; i<65; i=i+1) t=tuple(t);")
        .unwrap_err();
    assert!(error[0].message.contains("depth limit"));
    interpreter
        .run("t = tuple(nil); for (var i=0; i<16; i=i+1) t=concat(t,t);")
        .unwrap();
    assert_eq!(interpreter.run("print len(t);").unwrap(), ["65536"]);
    assert!(interpreter.run("t=append(t,nil);").unwrap_err()[0]
        .message
        .contains("item limit"));
    assert_eq!(interpreter.run("print len(t);").unwrap(), ["65536"]);
}

#[test]
fn shared_collection_dags_have_an_expanded_size_budget() {
    let error = Interpreter::new()
        .run("var t=nil; for (var i=0; i<20; i=i+1) t=tuple(t,t);")
        .unwrap_err();
    assert!(error[0].message.contains("expanded size limit"));
}

#[test]
fn diagnostic_excerpts_bound_unicode_and_sanitize_controls() {
    let diagnostic = Diagnostic::new("Runtime", 1, "failure");
    let excerpt = diagnostic.render(&"🦀".repeat(250));
    assert_eq!(excerpt.matches('🦀').count(), 200);
    assert!(excerpt.ends_with('…'));
    assert!(!diagnostic.render("\u{1b}\t").contains('\u{1b}'));
    assert_eq!(
        Diagnostic::new("Runtime", 0, "failure").render("source"),
        "[line 0] Runtime error: failure"
    );
}
