/**
 * Centralized Signals State Store for AQVN Workspace.
 * Uses @preact/signals-core (1.6KB) for fine-grained reactive state atoms.
 * Works seamlessly across browser webview and Node.js headless environments.
 */

(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define(['@preact/signals-core'], factory);
  } else if (typeof module === 'object' && module.exports) {
    var signalsCore;
    try {
      signalsCore = require('@preact/signals-core');
    } catch (e) {
      signalsCore = root.preactSignalsCore;
    }
    module.exports = factory(signalsCore);
  } else {
    var core = root.preactSignalsCore || {};
    root.AqvnSignals = factory(core);
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function (core) {
  'use strict';

  var signal = (core && core.signal) ? core.signal : function (v) {
    var val = v;
    var subs = new Set();
    return {
      get value() { return val; },
      set value(next) {
        if (val !== next) {
          val = next;
          subs.forEach(function (fn) { fn(val); });
        }
      },
      peek: function () { return val; },
      subscribe: function (fn) {
        subs.add(fn);
        fn(val);
        return function () { subs.delete(fn); };
      }
    };
  };

  var computed = (core && core.computed) ? core.computed : function (fn) {
    var sig = signal(fn());
    return {
      get value() { return fn(); },
      peek: function () { return fn(); },
      subscribe: function (cb) {
        return sig.subscribe(cb);
      }
    };
  };

  var effect = (core && core.effect) ? core.effect : function (fn) {
    fn();
    return function () {};
  };

  var batch = (core && core.batch) ? core.batch : function (fn) {
    return fn();
  };

  // 1. Session & Chat Domain Store
  var activeSessionId = signal(null);
  var sessionsList = signal([]);
  var messagesQueue = signal([]);
  var isStreaming = signal(false);

  // 2. Telemetry & Connection Domain Store
  var connectionState = signal('connected'); // 'connected' | 'reconnecting' | 'offline'
  var hostMetrics = signal({ cpu: 0, ramMb: 0, latencyMs: 0 });
  var pingHistory = signal([]);

  // Computed signal: Tự động tính toán trạng thái sức khỏe mạng
  var isNetworkHealthy = computed(function () {
    var state = connectionState.value;
    var metrics = hostMetrics.value;
    return state === 'connected' && (metrics && metrics.latencyMs < 100);
  });

  // 3. Workspace & Files Domain Store
  var currentWorkspacePath = signal('~/Workspace');
  var fileTreeNodes = signal([]);
  var activeEditingFile = signal(null);

  // 4. Settings & Preferences Domain Store
  var currentTheme = signal('aqvn-dark');
  var currentLang = signal('vi');
  var securityPolicy = signal('supervised');

  // 5. Shell Layout & Panels State Store
  var sidebarCollapsed = signal(false);
  var canvasCollapsed = signal(false);
  var focusMode = signal(false);
  var terminalOpen = signal(false);
  var searchOpen = signal(false);
  var activeSidebarTab = signal('sessions');

  var store = {
    // Core primitives
    signal: signal,
    computed: computed,
    effect: effect,
    batch: batch,

    // Session atoms
    activeSessionId: activeSessionId,
    sessionsList: sessionsList,
    messagesQueue: messagesQueue,
    isStreaming: isStreaming,

    // Telemetry atoms
    connectionState: connectionState,
    hostMetrics: hostMetrics,
    pingHistory: pingHistory,
    isNetworkHealthy: isNetworkHealthy,

    // Workspace atoms
    currentWorkspacePath: currentWorkspacePath,
    fileTreeNodes: fileTreeNodes,
    activeEditingFile: activeEditingFile,

    // Settings atoms
    currentTheme: currentTheme,
    currentLang: currentLang,
    securityPolicy: securityPolicy,

    // Shell atoms
    sidebarCollapsed: sidebarCollapsed,
    canvasCollapsed: canvasCollapsed,
    focusMode: focusMode,
    terminalOpen: terminalOpen,
    searchOpen: searchOpen,
    activeSidebarTab: activeSidebarTab
  };

  return store;
});
