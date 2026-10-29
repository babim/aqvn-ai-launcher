/**
 * AQVN Workspace Webview Container Bridge Script
 *
 * Injected into the webview container to monitor agent task state,
 * detect completion events from WebSocket/DOM state, update system tray
 * icon status (idle/busy), and dispatch native desktop notifications
 * (notify_task_complete) when the window is hidden or minimized.
 */

(function () {
  'use strict';

  function invokeIpc(command, args) {
    if (
      typeof window !== 'undefined' &&
      window.__TAURI_INTERNALS__ &&
      typeof window.__TAURI_INTERNALS__.invoke === 'function'
    ) {
      return window.__TAURI_INTERNALS__.invoke(command, args || {}).catch(function () {
        /* Ignore IPC errors in fallback or restricted environments */
      });
    }
    if (
      typeof window !== 'undefined' &&
      window.__TAURI__ &&
      window.__TAURI__.core &&
      typeof window.__TAURI__.core.invoke === 'function'
    ) {
      return window.__TAURI__.core.invoke(command, args || {}).catch(function () {});
    }
    return Promise.resolve();
  }

  var state = {
    busy: false,
    lastProgress: '',
    lastSummary: ''
  };

  function notifyTaskComplete(summary) {
    var body = summary || state.lastProgress || state.lastSummary || '';
    body = String(body).replace(/\s+/g, ' ').trim().slice(0, 140);
    if (!body) {
      body = 'The assistant finished responding.';
    }
    return invokeIpc('notify_task_complete', {
      title: 'Task complete',
      body: body
    });
  }

  function updateTrayBusy(busy) {
    return invokeIpc('set_tray_busy', { busy: Boolean(busy) });
  }

  function setTaskState(busy, summary) {
    var wasBusy = state.busy;
    state.busy = Boolean(busy);
    if (summary) {
      state.lastProgress = String(summary);
      state.lastSummary = String(summary);
    }

    if (state.busy !== wasBusy) {
      updateTrayBusy(state.busy);
      if (wasBusy && !state.busy) {
        if (typeof document !== 'undefined' && document.hidden === true) {
          notifyTaskComplete(summary);
        }
      }
    }
  }

  function handleWebSocketMessage(data) {
    if (typeof data !== 'string') return;

    // 1. Socket.IO engine.io packet format: 42["event", payload]
    if (data.startsWith('42')) {
      try {
        var parsed = JSON.parse(data.slice(2));
        if (Array.isArray(parsed) && parsed.length >= 2) {
          var evt = parsed[0];
          var payload = parsed[1];
          if (evt === 'log_progress' && payload) {
            setTaskState(payload.active === true, payload.progress);
            return;
          }
          if (evt === 'state_push' && payload) {
            var snapshot = (payload.data && payload.data.snapshot) || payload.snapshot;
            if (snapshot) {
              if (typeof snapshot.log_progress_active === 'boolean') {
                setTaskState(snapshot.log_progress_active, snapshot.log_progress);
                return;
              }
            }
          }
        }
      } catch (e) {}
    }

    // 2. Direct JSON payload format
    try {
      var json = JSON.parse(data);
      if (json && typeof json === 'object') {
        if (typeof json.log_progress_active === 'boolean') {
          setTaskState(json.log_progress_active, json.log_progress);
          return;
        }
        if (typeof json.active === 'boolean') {
          setTaskState(json.active, json.progress || json.message);
          return;
        }
        if (json.status === 'completed' || json.status === 'done' || json.event === 'task_complete') {
          setTaskState(false, json.message || json.result || json.summary);
          return;
        }
        if (json.status === 'running' || json.status === 'busy' || json.event === 'task_start') {
          setTaskState(true, json.message);
          return;
        }
      }
    } catch (e) {}
  }

  function checkDomState() {
    if (typeof document === 'undefined' || !document.body) return;

    var body = document.body;
    var isDomBusy = false;

    // 1. Check body class list for 'busy'
    if (body.classList && body.classList.contains('busy')) {
      isDomBusy = true;
    }

    // 2. Check aria-busy attributes
    if (body.getAttribute && body.getAttribute('aria-busy') === 'true') {
      isDomBusy = true;
    }
    var busyEl = document.querySelector && document.querySelector('[aria-busy="true"]');
    if (busyEl) {
      isDomBusy = true;
    }

    // 3. Check active stop button
    var stopBtn = (document.getElementById && document.getElementById('stop')) ||
                  (document.querySelector && document.querySelector('button.stop-btn, button#stop'));
    if (stopBtn && !stopBtn.disabled && !stopBtn.hidden && stopBtn.style && stopBtn.style.display !== 'none') {
      isDomBusy = true;
    }

    // 4. Check active status element
    var statusEl = (document.getElementById && document.getElementById('status')) ||
                   (document.querySelector && document.querySelector('.status, [data-status]'));
    var summary = '';
    if (statusEl && !statusEl.hidden && statusEl.textContent && statusEl.textContent.trim().length > 0) {
      summary = statusEl.textContent;
      if (!isDomBusy && statusEl.dataset && statusEl.dataset.active === 'true') {
        isDomBusy = true;
      }
    }

    if (state.busy !== isDomBusy) {
      setTaskState(isDomBusy, summary);
    }
  }

  function installWebSocketInterceptor() {
    if (typeof window === 'undefined') return;

    if (typeof window.WebSocket === 'function') {
      var OriginalWebSocket = window.WebSocket;
      var WrappedWebSocket = function (url, protocols) {
        var ws = protocols !== undefined
          ? new OriginalWebSocket(url, protocols)
          : new OriginalWebSocket(url);

        ws.addEventListener('message', function (event) {
          if (event && event.data) {
            handleWebSocketMessage(event.data);
          }
        });

        return ws;
      };
      WrappedWebSocket.prototype = OriginalWebSocket.prototype;
      WrappedWebSocket.CONNECTING = OriginalWebSocket.CONNECTING;
      WrappedWebSocket.OPEN = OriginalWebSocket.OPEN;
      WrappedWebSocket.CLOSING = OriginalWebSocket.CLOSING;
      WrappedWebSocket.CLOSED = OriginalWebSocket.CLOSED;
      window.WebSocket = WrappedWebSocket;
    }

    function hookIo(ioFn) {
      if (typeof ioFn !== 'function' || ioFn.__aqvn_hooked__) return ioFn;
      var wrappedIo = function () {
        var socket = ioFn.apply(this, arguments);
        if (socket && typeof socket.on === 'function') {
          socket.on('log_progress', function (payload) {
            if (payload) {
              setTaskState(payload.active === true, payload.progress);
            }
          });
          socket.on('state_push', function (payload) {
            if (payload) {
              var snapshot = (payload.data && payload.data.snapshot) || payload.snapshot;
              if (snapshot && typeof snapshot.log_progress_active === 'boolean') {
                setTaskState(snapshot.log_progress_active, snapshot.log_progress);
              }
            }
          });
        }
        return socket;
      };
      Object.assign(wrappedIo, ioFn);
      wrappedIo.__aqvn_hooked__ = true;
      return wrappedIo;
    }

    if (typeof window.io === 'function') {
      window.io = hookIo(window.io);
    } else {
      var _io = undefined;
      try {
        Object.defineProperty(window, 'io', {
          configurable: true,
          enumerable: true,
          get: function () {
            return _io;
          },
          set: function (fn) {
            _io = hookIo(fn);
          }
        });
      } catch (e) {}
    }
  }

  function installDomObserver() {
    if (typeof MutationObserver === 'undefined' || typeof document === 'undefined') return;

    var observer = new MutationObserver(function () {
      checkDomState();
    });

    function attach() {
      if (document.body) {
        observer.observe(document.body, {
          attributes: true,
          childList: true,
          subtree: true,
          attributeFilter: ['class', 'aria-busy', 'disabled', 'hidden', 'style']
        });
        checkDomState();
      } else if (document.addEventListener) {
        document.addEventListener('DOMContentLoaded', attach);
      }
    }

    attach();
  }

  function installFocusModeStyles() {
    if (typeof document === 'undefined') return;
    var css =
      'body.aqvn-focus-mode .sidebar-shell,\n' +
      'body.aqvn-focus-mode #left-panel,\n' +
      'body.aqvn-focus-mode .sidebar-slot,\n' +
      'body.aqvn-focus-mode #sidebar-header-panel,\n' +
      'body.aqvn-focus-mode #sidebar-overlay,\n' +
      'body.aqvn-focus-mode #right-canvas,\n' +
      'body.aqvn-focus-mode .right-canvas-rail,\n' +
      'body.aqvn-focus-mode #sidebar,\n' +
      'body.aqvn-focus-mode #canvas-pane,\n' +
      'body.aqvn-focus-mode .splitter {\n' +
      '  display: none !important;\n' +
      '}\n\n' +
      'body.aqvn-focus-mode #right-panel,\n' +
      'body.aqvn-focus-mode .chat-container,\n' +
      'body.aqvn-focus-mode #chat-pane {\n' +
      '  flex: 1 1 auto !important;\n' +
      '  width: 100% !important;\n' +
      '  max-width: 960px !important;\n' +
      '  margin: 0 auto !important;\n' +
      '}';

    function inject() {
      var root = document.head || document.body || document.documentElement;
      if (!root || typeof document.createElement !== 'function') return;
      if (document.getElementById && document.getElementById('aqvn-focus-mode-style')) return;
      var style = document.createElement('style');
      style.id = 'aqvn-focus-mode-style';
      style.textContent = css;
      root.appendChild(style);
    }

    if (document.head || document.body || document.documentElement) {
      inject();
    } else if (document.addEventListener) {
      document.addEventListener('DOMContentLoaded', inject);
    }
  }

  function toggleFocusMode() {
    if (typeof document === 'undefined' || !document.body) return false;
    installFocusModeStyles();
    var active = document.body.classList.toggle('aqvn-focus-mode');
    var input =
      (document.querySelector &&
        document.querySelector(
          '#chat-input, #message-input, textarea, input[type="text"]'
        )) ||
      null;
    if (input && typeof input.focus === 'function') {
      try {
        input.focus();
      } catch (e) {}
    }
    return active;
  }

  var ZOOM_KEY = 'aqvn_zoom';
  var ZOOM_MIN = 0.7;
  var ZOOM_MAX = 2.0;
  var ZOOM_STEP = 0.1;

  function clampZoom(value) {
    var n = value === null || value === undefined || value === '' ? 1 : Number(value);
    if (!isFinite(n)) n = 1;
    n = Math.round(n * 100) / 100;
    return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, n));
  }

  function applyZoom(level) {
    if (typeof document === 'undefined') return 1;
    var zoom = clampZoom(level);
    if (document.documentElement && document.documentElement.style) {
      document.documentElement.style.zoom = String(zoom);
    }
    if (document.body && document.body.style) {
      document.body.style.zoom = String(zoom);
    }
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(ZOOM_KEY, String(zoom));
      }
    } catch (err) {}
    return zoom;
  }

  function readZoom() {
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        return clampZoom(window.localStorage.getItem(ZOOM_KEY));
      }
    } catch (err) {}
    return 1;
  }

  function zoomIn() {
    return applyZoom(readZoom() + ZOOM_STEP);
  }

  function zoomOut() {
    return applyZoom(readZoom() - ZOOM_STEP);
  }

  function resetZoom() {
    return applyZoom(1);
  }

  function restoreZoom() {
    if (typeof document !== 'undefined' && (document.body || document.documentElement)) {
      applyZoom(readZoom());
    }
    if (typeof document !== 'undefined' && typeof document.addEventListener === 'function') {
      document.addEventListener('DOMContentLoaded', function () {
        applyZoom(readZoom());
      });
    }
  }

  function hardRefresh() {
    if (typeof location !== 'undefined' && typeof location.reload === 'function') {
      try {
        location.reload();
      } catch (e) {}
    } else if (
      typeof window !== 'undefined' &&
      window.location &&
      typeof window.location.reload === 'function'
    ) {
      try {
        window.location.reload();
      } catch (e) {}
    }
  }

  function installKeyboardShortcuts(dom) {
    function keyHandler(e) {
      var key = String(e.key || '').toLowerCase();
      var isCtrl = Boolean(e.ctrlKey || e.metaKey);

      if (!isCtrl && !e.altKey) {
        if (e.key === 'F1') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (
            typeof window !== 'undefined' &&
            window.__TAURI_INTERNALS__ &&
            typeof window.__TAURI_INTERNALS__.invoke === 'function'
          ) {
            window.__TAURI_INTERNALS__.invoke('open_help');
          } else {
            invokeIpc('open_help');
          }
          return;
        }
        if (e.key === 'F5') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          hardRefresh();
          return;
        }
        if (key === 'f11') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          toggleFocusMode();
          return;
        }
      }

      if (isCtrl && !e.altKey) {
        if (e.key === ',') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (
            typeof window !== 'undefined' &&
            window.__TAURI_INTERNALS__ &&
            typeof window.__TAURI_INTERNALS__.invoke === 'function'
          ) {
            window.__TAURI_INTERNALS__.invoke('open_settings');
          } else {
            invokeIpc('open_settings');
          }
          return;
        }
        if (e.shiftKey && (key === 'f' || e.code === 'KeyF')) {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          toggleFocusMode();
          return;
        }
        if (e.shiftKey && (key === 'r' || e.code === 'KeyR')) {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          hardRefresh();
          return;
        }
        // Zoom in: Ctrl+=, Ctrl++, or NumpadAdd (can have shiftKey or not)
        if (key === '=' || key === '+' || e.code === 'Equal' || e.code === 'NumpadAdd') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          zoomIn();
          return;
        }
        // Zoom out: Ctrl+-, Ctrl+_, or NumpadSubtract
        if (!e.shiftKey && (key === '-' || key === '_' || e.code === 'Minus' || e.code === 'NumpadSubtract')) {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          zoomOut();
          return;
        }
        // Reset zoom: Ctrl+0, Numpad0, or Digit0
        if (!e.shiftKey && (key === '0' || e.code === 'Digit0' || e.code === 'Numpad0')) {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          resetZoom();
          return;
        }
      }
    }

    function wheelHandler(e) {
      if (e.ctrlKey || e.metaKey) {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        if (e.deltaY < 0) {
          zoomIn();
        } else if (e.deltaY > 0) {
          zoomOut();
        }
      }
    }

    if (dom && typeof dom.addEventListener === 'function') {
      dom.addEventListener('keydown', keyHandler, true);
    } else {
      if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
        window.addEventListener('keydown', keyHandler, true);
        window.addEventListener('wheel', wheelHandler, { passive: false, capture: true });
      }
    }
  }

  var bridgeApi = {
    setTaskState: setTaskState,
    notifyTaskComplete: notifyTaskComplete,
    updateTrayBusy: updateTrayBusy,
    handleWebSocketMessage: handleWebSocketMessage,
    checkDomState: checkDomState,
    installKeyboardShortcuts: installKeyboardShortcuts,
    toggleFocusMode: toggleFocusMode,
    installFocusModeStyles: installFocusModeStyles,
    applyZoom: applyZoom,
    readZoom: readZoom,
    zoomIn: zoomIn,
    zoomOut: zoomOut,
    resetZoom: resetZoom,
    hardRefresh: hardRefresh,
    getState: function () {
      return {
        busy: state.busy,
        lastProgress: state.lastProgress,
        lastSummary: state.lastSummary
      };
    }
  };

  if (typeof window !== 'undefined') {
    window.__AQVN_CONTAINER__ = bridgeApi;
    window.__AQVN_BRIDGE__ = bridgeApi;
  }
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = bridgeApi;
  }

  installWebSocketInterceptor();
  installDomObserver();
  installKeyboardShortcuts();
  installFocusModeStyles();
  restoreZoom();
})();
