//! Hierarchical scopes example: demonstrates nested scope patterns with conject.
//!
//! conject's `#[scope]` creates child providers that wrap the parent, giving
//! child scopes access to everything the parent provides. This example shows:
//!
//! - Basic scoping (child accesses parent-provided types)
//! - Request-scoped injection with runtime arguments
//! - Named scopes for different lifetime categories
//! - Multiple independent named scopes on the same provider
#![allow(dead_code)]

use conject::{inject, injectable, provider};

// ---------------------------------------------------------------------------
// 1. Basic scope: child inherits parent provides
// ---------------------------------------------------------------------------

#[inject(Self(99))]
#[derive(Debug, PartialEq)]
struct RequestId(i32);

#[injectable]
#[derive(Debug)]
struct RequestHandler<'a> {
    id: &'a RequestId,
    #[inject(200)]
    status: i32,
}

#[provider]
#[provide(i32, 8080)]
#[scope(RequestId)]
struct BasicApp;

fn basic_scope_demo() {
    let app = BasicApp;
    let scope = app.scope();

    let handler: RequestHandler = scope.provide();
    println!(
        "[basic] RequestHandler: id={}, status={}",
        handler.id.0, handler.status
    );

    // Parent's i32 is also accessible from the scope
    let port: i32 = scope.provide();
    println!("[basic] Port from parent: {port}");
}

// ---------------------------------------------------------------------------
// 2. Request-scoped injection with runtime arguments
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct RequestContext {
    method: String,
    path: String,
    user_id: i32,
}

#[injectable]
#[derive(Debug)]
struct RouteHandler<'a> {
    ctx: &'a RequestContext,
    #[inject(200)]
    status_code: i32,
}

#[provider]
#[scope(#[arg] RequestContext)]
struct WebServer;

fn request_scope_demo() {
    let server = WebServer;

    // Each call to .scope() creates an isolated request scope
    let req1 = server.scope(RequestContext {
        method: "GET".into(),
        path: "/users".into(),
        user_id: 1,
    });
    let h1: RouteHandler = req1.provide();
    println!(
        "[request] {} {} by user {} -> {}",
        h1.ctx.method, h1.ctx.path, h1.ctx.user_id, h1.status_code
    );

    let req2 = server.scope(RequestContext {
        method: "POST".into(),
        path: "/orders".into(),
        user_id: 2,
    });
    let h2: RouteHandler = req2.provide();
    println!(
        "[request] {} {} by user {} -> {}",
        h2.ctx.method, h2.ctx.path, h2.ctx.user_id, h2.status_code
    );
}

// ---------------------------------------------------------------------------
// 3. Named scopes: independent scope categories on one provider
// ---------------------------------------------------------------------------

#[injectable]
#[derive(Debug)]
struct RequestService<'a> {
    path: &'a String,
    #[inject(200)]
    status: i32,
}

#[injectable]
#[derive(Debug)]
struct BackgroundJob<'a> {
    task_id: &'a u64,
    #[inject(3)]
    max_retries: i32,
}

#[provider]
#[scope(request: #[arg] String)]
#[scope(background: #[arg] u64)]
struct AppServer;

fn named_scopes_demo() {
    let server = AppServer;

    // Request scope
    let req = server.request_scope("GET /api/health".into());
    let svc: RequestService = req.provide();
    println!("[named] Request: path={}, status={}", svc.path, svc.status);

    // Background scope (completely independent)
    let bg = server.background_scope(42);
    let job: BackgroundJob = bg.provide();
    println!(
        "[named] Background: task_id={}, max_retries={}",
        job.task_id, job.max_retries
    );
}

// ---------------------------------------------------------------------------
// 4. Scope with parent field provides
// ---------------------------------------------------------------------------

#[inject(Self("v1.0".to_string()))]
#[derive(Debug, PartialEq)]
struct ApiVersion(String);

#[injectable]
#[derive(Debug)]
struct VersionedHandler<'a> {
    version: &'a ApiVersion,
    port: &'a u16,
}

#[provider]
#[scope(ApiVersion)]
struct ConfiguredApp(#[provide] u16);

fn field_provide_scope_demo() {
    let app = ConfiguredApp(3000);
    let scope = app.scope();

    let handler: VersionedHandler = scope.provide();
    println!(
        "[field] version={}, port={}",
        handler.version.0, handler.port
    );
}

// ---------------------------------------------------------------------------

fn main() {
    println!("=== Hierarchical Scopes Demo ===\n");

    println!("--- 1. Basic scope (child inherits parent) ---");
    basic_scope_demo();

    println!("\n--- 2. Request-scoped injection ---");
    request_scope_demo();

    println!("\n--- 3. Named scopes ---");
    named_scopes_demo();

    println!("\n--- 4. Scope with parent field provides ---");
    field_provide_scope_demo();

    println!("\n=== Done ===");
}
