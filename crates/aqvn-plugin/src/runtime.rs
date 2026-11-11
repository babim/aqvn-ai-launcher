//! Wasmtime runtime engine with Fuel Metering, Epoch Interruption, and WASI Preview 2.

use crate::config::PluginConfig;
use crate::error::{PluginError, PluginResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use wasmtime::component::Linker as ComponentLinker;
use wasmtime::{Config, Engine, Linker as CoreLinker, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

/// Host state encapsulated within each guest execution `Store`.
pub struct PluginHostState {
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    pub limits: StoreLimits,
    pub plugin_id: String,
    pub initial_fuel: u64,
    pub epoch_deadline_ticks: u64,
}

impl WasiView for PluginHostState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

/// Background epoch timer thread handle to trigger execution interrupts.
pub struct EpochTimer {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl EpochTimer {
    pub fn start(engine: Engine, interval: Duration) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = std::thread::Builder::new()
            .name("aqvn-wasm-epoch-timer".to_string())
            .spawn(move || {
                while running_clone.load(Ordering::Relaxed) {
                    std::thread::sleep(interval);
                    engine.increment_epoch();
                }
            })
            .ok();

        Self { running, handle }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EpochTimer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Central WASM Component Model and Core Module Runtime.
#[derive(Clone)]
pub struct WasmPluginEngine {
    engine: Engine,
    config: PluginConfig,
    _epoch_timer: Arc<std::sync::Mutex<EpochTimer>>,
}

impl WasmPluginEngine {
    /// Initialize a new plugin engine with the given configuration.
    pub fn new(config: PluginConfig) -> PluginResult<Self> {
        let mut wasm_cfg = Config::new();
        wasm_cfg.wasm_component_model(true);

        if config.enable_fuel_metering {
            wasm_cfg.consume_fuel(true);
        }

        if config.enable_epoch_interruption {
            wasm_cfg.epoch_interruption(true);
        }

        wasm_cfg.max_wasm_stack(config.max_stack_bytes);

        let engine = Engine::new(&wasm_cfg)
            .map_err(|e| PluginError::Wasm(format!("Failed to initialize Wasmtime engine: {e}")))?;

        let epoch_timer = EpochTimer::start(engine.clone(), config.epoch_tick_interval());

        Ok(Self {
            engine,
            config,
            _epoch_timer: Arc::new(std::sync::Mutex::new(epoch_timer)),
        })
    }

    /// Access the underlying Wasmtime `Engine`.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Access the active `PluginConfig`.
    pub fn config(&self) -> &PluginConfig {
        &self.config
    }

    /// Manually increment the epoch counter by 1.
    pub fn increment_epoch(&self) {
        self.engine.increment_epoch();
    }

    /// Construct a new `Store` configured with fuel metering, epoch deadline, and memory limits.
    pub fn create_store(
        &self,
        plugin_id: &str,
        override_fuel: Option<u64>,
        override_timeout_ms: Option<u64>,
    ) -> PluginResult<Store<PluginHostState>> {
        let fuel_limit = override_fuel.unwrap_or(self.config.default_fuel_limit);
        let timeout_ms = override_timeout_ms.unwrap_or(self.config.default_epoch_timeout_ms);

        let tick_ms = self.config.epoch_tick_interval_ms.max(1);
        let deadline_ticks = (timeout_ms / tick_ms).max(1);

        let mut wasi_builder = WasiCtxBuilder::new();
        wasi_builder.inherit_stderr();

        let limits = StoreLimitsBuilder::new()
            .memory_size(self.config.max_memory_bytes)
            .build();

        let state = PluginHostState {
            wasi_ctx: wasi_builder.build(),
            table: ResourceTable::new(),
            limits,
            plugin_id: plugin_id.to_string(),
            initial_fuel: fuel_limit,
            epoch_deadline_ticks: deadline_ticks,
        };

        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);

        if self.config.enable_fuel_metering {
            store.set_fuel(fuel_limit).map_err(|e| {
                PluginError::Wasm(format!("Failed to set initial execution fuel: {e}"))
            })?;
        }

        if self.config.enable_epoch_interruption {
            store.set_epoch_deadline(deadline_ticks);
        }

        Ok(store)
    }

    /// Calculate fuel consumed during execution.
    pub fn get_consumed_fuel(&self, store: &mut Store<PluginHostState>) -> PluginResult<u64> {
        if !self.config.enable_fuel_metering {
            return Ok(0);
        }
        let remaining = store.get_fuel().map_err(|e| {
            PluginError::Wasm(format!("Failed to read remaining fuel from store: {e}"))
        })?;
        let initial = store.data().initial_fuel;
        Ok(initial.saturating_sub(remaining))
    }

    /// Create a WASI Preview 2 enabled Component Linker.
    pub fn create_component_linker(&self) -> PluginResult<ComponentLinker<PluginHostState>> {
        let mut linker = ComponentLinker::new(&self.engine);
        if self.config.enable_wasi {
            wasmtime_wasi::add_to_linker_sync(&mut linker).map_err(|e| {
                PluginError::Wasm(format!("Failed to link WASI Preview 2 interfaces: {e}"))
            })?;
        }
        Ok(linker)
    }

    /// Create a Core Module Linker.
    pub fn create_core_linker(&self) -> PluginResult<CoreLinker<PluginHostState>> {
        let linker = CoreLinker::new(&self.engine);
        Ok(linker)
    }
}
