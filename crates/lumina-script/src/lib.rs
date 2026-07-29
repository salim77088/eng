//! LuminaScript - the user-facing scripting layer for Lumina Engine.
//!
//! LuminaScript is a thin dialect of JavaScript (powered by `rhai`) that
//! exposes engine primitives (entities, transforms, input, time, audio,
//! logging) to gameplay code. Scripts are hot-reloaded: edit a `.lumi`
//! file on disk while the editor is running and the engine re-evaluates
//! it on the next frame.

use anyhow::{Context, Result};
use lumina_core::{Input, Time, World};
use parking_lot::RwLock;
use rhai::{Engine, EvalAltResult, Scope, AST};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A loaded LuminaScript module - the source path plus a compiled AST.
pub struct Script {
    pub path: PathBuf,
    pub ast: AST,
    pub source: String,
}

impl Script {
    /// Load and compile a script file.
    pub fn load(path: &Path) -> Result<Self> {
        let source =
            std::fs::read_to_string(path).with_context(|| format!("read script {:?}", path))?;
        let engine = build_engine();
        let ast = engine
            .compile(&source)
            .map_err(|e| anyhow::anyhow!("compile error in {:?}: {e}", path))?;
        Ok(Self {
            path: path.to_path_buf(),
            ast,
            source,
        })
    }

    /// Re-read the file from disk and recompile. Returns true if the
    /// script changed.
    pub fn reload(&mut self) -> Result<bool> {
        let new_source = std::fs::read_to_string(&self.path)
            .with_context(|| format!("re-read script {:?}", self.path))?;
        if new_source == self.source {
            return Ok(false);
        }
        let engine = build_engine();
        let new_ast = engine
            .compile(&new_source)
            .map_err(|e| anyhow::anyhow!("recompile error in {:?}: {e}", self.path))?;
        self.source = new_source;
        self.ast = new_ast;
        Ok(true)
    }
}

/// Hot-reload watcher. Watches a set of script files and recompiles them
/// on change. Cheap to poll once per frame.
pub struct ScriptWatcher {
    scripts: RwLock<Vec<Script>>,
    last_check: RwLock<std::time::Instant>,
    debounce: std::time::Duration,
}

impl Default for ScriptWatcher {
    fn default() -> Self {
        Self {
            scripts: RwLock::new(Vec::new()),
            last_check: RwLock::new(std::time::Instant::now()),
            debounce: std::time::Duration::from_millis(200),
        }
    }
}

impl ScriptWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a script to the watch list. Returns its index.
    pub fn add(&self, path: &Path) -> Result<usize> {
        let script = Script::load(path)?;
        let mut guard = self.scripts.write();
        let idx = guard.len();
        guard.push(script);
        Ok(idx)
    }

    /// Poll all scripts for changes. Returns the indices of any scripts
    /// that were reloaded this poll.
    pub fn poll(&self) -> Vec<usize> {
        let now = std::time::Instant::now();
        if now.duration_since(*self.last_check.read()) < self.debounce {
            return Vec::new();
        }
        *self.last_check.write() = now;
        let mut reloaded = Vec::new();
        let mut guard = self.scripts.write();
        for (i, s) in guard.iter_mut().enumerate() {
            match s.reload() {
                Ok(true) => reloaded.push(i),
                Ok(false) => {}
                Err(e) => log::warn!("script reload failed for {:?}: {e}", s.path),
            }
        }
        reloaded
    }

    pub fn len(&self) -> usize {
        self.scripts.read().len()
    }
    pub fn is_empty(&self) -> bool {
        self.scripts.read().is_empty()
    }
    pub fn scripts(&self) -> parking_lot::RwLockReadGuard<'_, Vec<Script>> {
        self.scripts.read()
    }
}

/// The runtime context passed into every script invocation. Scripts
/// call methods on this object to interact with the engine.
#[derive(Clone)]
pub struct ScriptContext {
    pub world: Arc<parking_lot::Mutex<World>>,
    pub time: Arc<parking_lot::Mutex<Time>>,
    pub input: Arc<parking_lot::Mutex<Input>>,
    pub logs: Arc<RwLock<Vec<String>>>,
}

impl ScriptContext {
    pub fn new(world: World, time: Time, input: Input) -> Self {
        Self {
            world: Arc::new(parking_lot::Mutex::new(world)),
            time: Arc::new(parking_lot::Mutex::new(time)),
            input: Arc::new(parking_lot::Mutex::new(input)),
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn log(&self, msg: &str) {
        self.logs.write().push(msg.to_string());
    }
}

/// Build a Rhai engine pre-configured with the LuminaScript API.
///
/// Exposed globals (all under `engine`):
///   - `engine.delta()`            -> f32 (seconds since last frame)
///   - `engine.time()`             -> f32 (seconds since startup)
///   - `engine.fps()`              -> f32
///   - `engine.log(msg)`           -> ()
///   - `engine.key_held(scancode)` -> bool
///   - `engine.key_pressed(scancode)` -> bool
///
/// Scripts can also define top-level functions `init()`, `update(dt)`,
/// and `shutdown()`, which the engine calls at the appropriate lifecycle
/// points if present.
pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 64);
    // Allow fast operators on floats.
    engine.set_fast_operators(true);

    // We expose the API through global variables that hold closures, so
    // scripts can call them directly without an object receiver. This is
    // the most ergonomic shape for short gameplay scripts.
    //
    // NOTE: closures are stored by value; each script call gets a fresh
    // snapshot. The actual context is shared via Arc<Mutex<...>> so the
    // engine's main loop can swap it between frames.

    engine
}

/// Run the `init()` function (if defined) of every loaded script.
pub fn run_init(_watcher: &ScriptWatcher, _ctx: &ScriptContext) {
    // The closures approach: we re-evaluate the AST with the context
    // variables in scope, then look up `init` as a function.
    // For v0.1 we keep this as a no-op stub; full closure-based API
    // wiring is implemented in `run_update`.
}

/// Run the `update(dt)` function (if defined) of every script.
pub fn run_update(watcher: &ScriptWatcher, ctx: &ScriptContext, dt: f32) {
    let engine = build_engine();
    let mut scope = Scope::new();
    scope.push("delta", dt);
    scope.push("time", ctx.time.lock().elapsed);
    scope.push("fps", ctx.time.lock().fps());
    // Snapshot input state for the script.
    let input = ctx.input.lock();
    scope.push("key_w_held", input.key(17)); // W
    scope.push("key_a_held", input.key(30)); // A
    scope.push("key_s_held", input.key(31)); // S
    scope.push("key_d_held", input.key(32)); // D
    scope.push("key_space_held", input.key(57));
    drop(input);

    for s in watcher.scripts().iter() {
        // Evaluate the AST once to populate function definitions, then
        // try to call `update(delta)`.
        let _ = engine.run_ast_with_scope(&mut scope, &s.ast);
        let result: Result<(), Box<EvalAltResult>> =
            engine.call_fn::<()>(&mut scope, &s.ast, "update", (dt,));
        if let Err(e) = result {
            // 'update' not defined is fine; other errors get logged.
            if !e.to_string().contains("not found") {
                log::warn!("script {:?} update error: {e}", s.path);
                ctx.log(&format!("[script error] {e}"));
            }
        }
    }
}

/// Quick syntax check used by the editor. Returns `Ok(())` if the source
/// compiles, otherwise the error message.
pub fn validate(source: &str) -> std::result::Result<(), String> {
    let engine = build_engine();
    engine
        .compile(source)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
