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

  var bridgeApi = {
    setTaskState: setTaskState,
    notifyTaskComplete: notifyTaskComplete,
    updateTrayBusy: updateTrayBusy,
    handleWebSocketMessage: handleWebSocketMessage,
    checkDomState: checkDomState,
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
})();
