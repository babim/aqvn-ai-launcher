/**
 * Bridge layer for AQVN Workspace.
 * Automatically wraps Tauri v2 IPC when running inside the desktop app,
 * and falls back gracefully to in-browser mocks (e.g. localStorage for store)
 * so tests can run headless in agent-browser or standard browsers without errors.
 */

(function () {
  'use strict';

  function checkIsTauri() {
    return typeof window !== 'undefined' && Boolean(
      window.__TAURI_INTERNALS__ || window.isTauri || window.__TAURI__
    );
  }

  class BrowserStoreMock {
    constructor(path) {
      this.path = path;
      this.storageKey = 'aqvn_store_' + path;
      this._data = this._loadFromStorage();
    }

    _loadFromStorage() {
      try {
        const raw = localStorage.getItem(this.storageKey);
        return raw ? JSON.parse(raw) : {};
      } catch (e) {
        return {};
      }
    }

    _commitToStorage() {
      try {
        localStorage.setItem(this.storageKey, JSON.stringify(this._data));
      } catch (e) {
        // localStorage might be unavailable in rare sandbox environments
      }
    }

    async get(key) {
      return Object.prototype.hasOwnProperty.call(this._data, key)
        ? this._data[key]
        : undefined;
    }

    async set(key, value) {
      this._data[key] = value;
      this._commitToStorage();
    }

    async has(key) {
      return Object.prototype.hasOwnProperty.call(this._data, key);
    }

    async delete(key) {
      const existed = Object.prototype.hasOwnProperty.call(this._data, key);
      delete this._data[key];
      this._commitToStorage();
      return existed;
    }

    async clear() {
      this._data = {};
      this._commitToStorage();
    }

    async keys() {
      return Object.keys(this._data);
    }

    async values() {
      return Object.values(this._data);
    }

    async entries() {
      return Object.entries(this._data);
    }

    async save() {
      this._commitToStorage();
    }
  }

  class TauriStoreWrapper {
    constructor(path, rid, rawStore = null) {
      this.path = path;
      this.rid = rid;
      this.rawStore = rawStore;
    }

    static async load(path) {
      if (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.store && typeof window.__TAURI__.store.load === 'function') {
        try {
          const rawStore = await window.__TAURI__.store.load(path);
          return new TauriStoreWrapper(path, null, rawStore);
        } catch (e) {
          // Fall back to direct __TAURI_INTERNALS__.invoke below
        }
      }

      const rid = await window.__TAURI_INTERNALS__.invoke('plugin:store|load', {
        path: path
      });
      return new TauriStoreWrapper(path, rid);
    }

    async get(key) {
      if (this.rawStore) {
        return await this.rawStore.get(key);
      }
      const [value, exists] = await window.__TAURI_INTERNALS__.invoke('plugin:store|get', {
        rid: this.rid,
        key: key
      });
      return exists ? value : undefined;
    }

    async set(key, value) {
      if (this.rawStore) {
        return await this.rawStore.set(key, value);
      }
      await window.__TAURI_INTERNALS__.invoke('plugin:store|set', {
        rid: this.rid,
        key: key,
        value: value
      });
    }

    async has(key) {
      if (this.rawStore) {
        return await this.rawStore.has(key);
      }
      return await window.__TAURI_INTERNALS__.invoke('plugin:store|has', {
        rid: this.rid,
        key: key
      });
    }

    async delete(key) {
      if (this.rawStore) {
        return await this.rawStore.delete(key);
      }
      return await window.__TAURI_INTERNALS__.invoke('plugin:store|delete', {
        rid: this.rid,
        key: key
      });
    }

    async clear() {
      if (this.rawStore) {
        return await this.rawStore.clear();
      }
      await window.__TAURI_INTERNALS__.invoke('plugin:store|clear', {
        rid: this.rid
      });
    }

    async keys() {
      if (this.rawStore) {
        return await this.rawStore.keys();
      }
      return await window.__TAURI_INTERNALS__.invoke('plugin:store|keys', {
        rid: this.rid
      });
    }

    async values() {
      if (this.rawStore) {
        return await this.rawStore.values();
      }
      return await window.__TAURI_INTERNALS__.invoke('plugin:store|values', {
        rid: this.rid
      });
    }

    async entries() {
      if (this.rawStore) {
        return await this.rawStore.entries();
      }
      return await window.__TAURI_INTERNALS__.invoke('plugin:store|entries', {
        rid: this.rid
      });
    }

    async save() {
      if (this.rawStore) {
        return await this.rawStore.save();
      }
      await window.__TAURI_INTERNALS__.invoke('plugin:store|save', {
        rid: this.rid
      });
    }
  }

  const storeCache = new Map();

  async function getStore(path = 'settings.json') {
    if (storeCache.has(path)) {
      return storeCache.get(path);
    }

    let store;
    if (checkIsTauri()) {
      store = await TauriStoreWrapper.load(path);
    } else {
      store = new BrowserStoreMock(path);
    }

    storeCache.set(path, store);
    return store;
  }

  function backendDescription() {
    if (!checkIsTauri()) {
      return 'browser (localStorage)';
    }
    if (
      typeof window !== 'undefined' &&
      window.__TAURI__ &&
      window.__TAURI__.store &&
      typeof window.__TAURI__.store.load === 'function'
    ) {
      return 'tauri (global store api)';
    }
    return 'tauri (internals invoke)';
  }

  /**
   * Bridge proxy endpoint injected by Rust before any page script runs.
   * The port is chosen at launch, so it must never be hardcoded here.
   * Browser-mode (agent-browser against a static server) returns null.
   */
  function proxyInfo() {
    const injected = typeof window !== 'undefined' ? window.__AQVN_PROXY__ : null;
    if (!injected || typeof injected.port !== 'number') {
      return null;
    }
    return { url: String(injected.url), port: injected.port };
  }

  window.bridge = {
    isTauri: checkIsTauri(),
    getStore,
    backendDescription,
    proxy: proxyInfo()
  };
})();
