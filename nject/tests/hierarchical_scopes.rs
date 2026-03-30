#![allow(dead_code)]
use nject::{inject, injectable, provider};

#[test]
fn scope_can_provide_parent_types() {
    #[injectable]
    struct Config(#[inject(8080)] i32);

    #[injectable]
    struct RequestId(#[inject(42)] i32);

    #[provider]
    #[provide(i32, 3000)]
    #[scope(RequestId)]
    struct AppProvider;

    let app = AppProvider;
    let scope = app.scope();

    // Scope can provide its own type
    let req_id: RequestId = scope.provide();
    assert_eq!(req_id.0, 42);

    // Scope can also provide parent's types (i32 provided by parent)
    let port: i32 = scope.provide();
    assert_eq!(port, 3000);
}

#[test]
fn scope_with_args_simulates_request_scope() {
    #[derive(Debug, PartialEq)]
    struct RequestContext {
        user_id: i32,
        path: String,
    }

    #[injectable]
    struct Handler<'a> {
        ctx: &'a RequestContext,
        #[inject(200)]
        status: i32,
    }

    #[provider]
    #[scope(#[arg] RequestContext)]
    struct AppProvider;

    let app = AppProvider;

    // Simulate different requests
    let req1 = app.scope(RequestContext {
        user_id: 1,
        path: "/home".into(),
    });
    let handler1: Handler = req1.provide();
    assert_eq!(handler1.ctx.user_id, 1);
    assert_eq!(handler1.ctx.path, "/home");
    assert_eq!(handler1.status, 200);

    let req2 = app.scope(RequestContext {
        user_id: 2,
        path: "/api".into(),
    });
    let handler2: Handler = req2.provide();
    assert_eq!(handler2.ctx.user_id, 2);
}

#[test]
fn nested_scopes_access_all_ancestors() {
    // Root provides a value type and has a scope with an arg
    #[inject(Self(123))]
    #[derive(Debug, PartialEq)]
    struct AppId(i32);

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct ScopedValue<'a>(&'a AppId, &'a i32);

    #[provider]
    #[scope(AppId)]
    #[scope(#[arg] i32)]
    struct RootProvider;

    let root = RootProvider;
    let child = root.scope(42);

    // Child can access scope's own AppId
    let app_id: &AppId = child.provide();
    assert_eq!(*app_id, AppId(123));

    // Child can access the arg value
    let val: &i32 = child.provide();
    assert_eq!(*val, 42);

    // Child can provide a type that depends on both scope values
    let combined: ScopedValue = child.provide();
    assert_eq!(combined, ScopedValue(&AppId(123), &42));
}

#[test]
fn scope_with_named_scopes() {
    #[provider]
    #[scope(request: #[arg] String)]
    #[scope(session: #[arg] u64)]
    struct AppProvider;

    let app = AppProvider;

    let req = app.request_scope("GET /api".into());
    let path: &String = req.provide();
    assert_eq!(path, "GET /api");

    let sess = app.session_scope(12345);
    let id: &u64 = sess.provide();
    assert_eq!(*id, 12345);
}

#[test]
fn scope_inherits_parent_field_provides() {
    // Demonstrates that #[provide] on a field is accessible from scope
    #[inject(Self(99))]
    #[derive(Debug, PartialEq)]
    struct ScopedType(i32);

    #[injectable]
    #[derive(Debug, PartialEq)]
    struct Combined<'a>(&'a i32, &'a ScopedType);

    #[provider]
    #[scope(ScopedType)]
    struct AppProvider(#[provide] i32);

    let app = AppProvider(8080);
    let scope = app.scope();

    // Scope can access the parent's field-based provide
    let port: &i32 = scope.provide();
    assert_eq!(*port, 8080);

    // Scope also has its own type
    let scoped: &ScopedType = scope.provide();
    assert_eq!(*scoped, ScopedType(99));

    // Can combine both in a single injectable
    let combined: Combined = scope.provide();
    assert_eq!(combined, Combined(&8080, &ScopedType(99)));
}

#[test]
fn multiple_named_scopes_are_independent() {
    // Each named scope is independent - they don't share state
    #[provider]
    #[provide(i32, 42)]
    #[scope(request: #[arg] String)]
    #[scope(background: #[arg] u32)]
    struct Server;

    let server = Server;

    // Request scope gets string arg + parent's i32
    let req = server.request_scope("POST /users".into());
    let path: &String = req.provide();
    assert_eq!(path, "POST /users");
    let base_val: i32 = req.provide();
    assert_eq!(base_val, 42);

    // Background scope gets u32 arg + parent's i32
    let bg = server.background_scope(5);
    let retries: &u32 = bg.provide();
    assert_eq!(*retries, 5);
    let base_val: i32 = bg.provide();
    assert_eq!(base_val, 42);
}
