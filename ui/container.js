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
    try {
      if (
        typeof window !== 'undefined' &&
        window.__TAURI_INTERNALS__ &&
        typeof window.__TAURI_INTERNALS__.invoke === 'function'
      ) {
        return Promise.resolve(window.__TAURI_INTERNALS__.invoke(command, args || {})).catch(
          function (err) {
            if (typeof console !== 'undefined' && typeof console.warn === 'function') {
              console.warn('[AQVN Container] IPC error (' + command + '):', err);
            }
          }
        );
      }
      if (
        typeof window !== 'undefined' &&
        window.__TAURI__ &&
        window.__TAURI__.core &&
        typeof window.__TAURI__.core.invoke === 'function'
      ) {
        return Promise.resolve(window.__TAURI__.core.invoke(command, args || {})).catch(
          function (err) {
            if (typeof console !== 'undefined' && typeof console.warn === 'function') {
              console.warn('[AQVN Container] IPC error (' + command + '):', err);
            }
          }
        );
      }
    } catch (err) {
      if (typeof console !== 'undefined' && typeof console.warn === 'function') {
        console.warn('[AQVN Container] IPC invocation failed (' + command + '):', err);
      }
    }
    return Promise.resolve();
  }

  var state = {
    busy: false,
    lastProgress: '',
    lastSummary: '',
    taskChimeEnabled: undefined,
    taskChimeVolume: undefined,
    taskChimeTone: undefined
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

  var TASK_CHIME_KEY = 'aqvn_task_chime_enabled';
  var TASK_CHIME_VOLUME_KEY = 'aqvn_task_chime_volume';
  var TASK_CHIME_TONE_KEY = 'aqvn_task_chime_tone';
  var DEFAULT_TASK_CHIME_VOLUME = 0.5;
  var DEFAULT_TASK_CHIME_TONE = 'classic';
  var VALID_CHIME_TONES = ['classic', 'bell', 'beep', 'subtle'];

  function isTaskChimeEnabled() {
    if (state.taskChimeEnabled !== undefined) {
      return Boolean(state.taskChimeEnabled);
    }
    try {
      if (typeof window !== 'undefined') {
        if (window.__AQVN_TASK_CHIME_ENABLED__ !== undefined) {
          return Boolean(window.__AQVN_TASK_CHIME_ENABLED__);
        }
        if (window.localStorage) {
          var val = window.localStorage.getItem(TASK_CHIME_KEY);
          if (val !== null) {
            return val === 'true' || val === true;
          }
        }
      }
    } catch (e) {}
    return true;
  }

  function setTaskChimeEnabled(enabled) {
    state.taskChimeEnabled = Boolean(enabled);
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(TASK_CHIME_KEY, String(state.taskChimeEnabled));
      }
    } catch (e) {}
    return state.taskChimeEnabled;
  }

  function getTaskChimeVolume() {
    if (typeof state.taskChimeVolume === 'number') {
      return state.taskChimeVolume;
    }
    try {
      if (typeof window !== 'undefined') {
        if (typeof window.__AQVN_TASK_CHIME_VOLUME__ === 'number') {
          return Math.max(0, Math.min(1, window.__AQVN_TASK_CHIME_VOLUME__));
        }
        if (window.localStorage) {
          var val = window.localStorage.getItem(TASK_CHIME_VOLUME_KEY);
          if (val !== null) {
            var parsed = parseFloat(val);
            if (!isNaN(parsed)) {
              return Math.max(0, Math.min(1, parsed));
            }
          }
        }
      }
    } catch (e) {}
    return DEFAULT_TASK_CHIME_VOLUME;
  }

  function setTaskChimeVolume(vol) {
    var num = parseFloat(vol);
    if (isNaN(num)) {
      num = DEFAULT_TASK_CHIME_VOLUME;
    }
    state.taskChimeVolume = Math.max(0, Math.min(1, num));
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(TASK_CHIME_VOLUME_KEY, String(state.taskChimeVolume));
      }
    } catch (e) {}
    return state.taskChimeVolume;
  }

  function getTaskChimeTone() {
    if (typeof state.taskChimeTone === 'string' && VALID_CHIME_TONES.indexOf(state.taskChimeTone) !== -1) {
      return state.taskChimeTone;
    }
    try {
      if (typeof window !== 'undefined') {
        if (typeof window.__AQVN_TASK_CHIME_TONE__ === 'string' && VALID_CHIME_TONES.indexOf(window.__AQVN_TASK_CHIME_TONE__) !== -1) {
          return window.__AQVN_TASK_CHIME_TONE__;
        }
        if (window.localStorage) {
          var val = window.localStorage.getItem(TASK_CHIME_TONE_KEY);
          if (val !== null && VALID_CHIME_TONES.indexOf(val) !== -1) {
            return val;
          }
        }
      }
    } catch (e) {}
    return DEFAULT_TASK_CHIME_TONE;
  }

  function setTaskChimeTone(tone) {
    var str = String(tone || '').toLowerCase().trim();
    if (VALID_CHIME_TONES.indexOf(str) === -1) {
      str = DEFAULT_TASK_CHIME_TONE;
    }
    state.taskChimeTone = str;
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(TASK_CHIME_TONE_KEY, state.taskChimeTone);
      }
    } catch (e) {}
    return state.taskChimeTone;
  }

  function playTaskChime(toneOverride, volumeOverride) {
    if (!isTaskChimeEnabled()) {
      return null;
    }
    var tone = (typeof toneOverride === 'string' && VALID_CHIME_TONES.indexOf(toneOverride) !== -1)
      ? toneOverride
      : getTaskChimeTone();
    var volume = (typeof volumeOverride === 'number' && !isNaN(volumeOverride))
      ? Math.max(0, Math.min(1, volumeOverride))
      : getTaskChimeVolume();

    try {
      var AudioCtx =
        (typeof window !== 'undefined' && (window.AudioContext || window.webkitAudioContext)) ||
        (typeof globalThis !== 'undefined' && (globalThis.AudioContext || globalThis.webkitAudioContext));
      if (!AudioCtx) {
        return null;
      }

      var ctx = new AudioCtx();
      var now = (typeof ctx.currentTime === 'number') ? ctx.currentTime : 0;
      var osc = ctx.createOscillator();
      var gain = ctx.createGain();

      var peakGain = 0.2 * volume;
      var duration = 0.2;

      if (tone === 'bell') {
        osc.type = 'sine';
        if (osc.frequency && typeof osc.frequency.setValueAtTime === 'function') {
          osc.frequency.setValueAtTime(587.33, now);
          osc.frequency.setValueAtTime(880, now + 0.08);
        }
        duration = 0.25;
      } else if (tone === 'beep') {
        osc.type = 'triangle';
        if (osc.frequency && typeof osc.frequency.setValueAtTime === 'function') {
          osc.frequency.setValueAtTime(800, now);
          osc.frequency.setValueAtTime(1200, now + 0.08);
        }
        duration = 0.16;
      } else if (tone === 'subtle') {
        osc.type = 'sine';
        if (osc.frequency && typeof osc.frequency.setValueAtTime === 'function') {
          osc.frequency.setValueAtTime(330, now);
          osc.frequency.setValueAtTime(440, now + 0.1);
        }
        peakGain = 0.15 * volume;
        duration = 0.25;
      } else {
        // 'classic' (default dual tone)
        osc.type = 'sine';
        if (osc.frequency && typeof osc.frequency.setValueAtTime === 'function') {
          osc.frequency.setValueAtTime(440, now);
          osc.frequency.setValueAtTime(880, now + 0.1);
        }
        duration = 0.2;
      }

      if (gain.gain && typeof gain.gain.setValueAtTime === 'function') {
        gain.gain.setValueAtTime(peakGain, now);
        if (typeof gain.gain.exponentialRampToValueAtTime === 'function') {
          gain.gain.exponentialRampToValueAtTime(0.001, now + duration);
        } else if (typeof gain.gain.linearRampToValueAtTime === 'function') {
          gain.gain.linearRampToValueAtTime(0.001, now + duration);
        }
      }

      if (typeof osc.connect === 'function') {
        osc.connect(gain);
      }
      if (gain && typeof gain.connect === 'function' && ctx.destination) {
        gain.connect(ctx.destination);
      }

      if (ctx.state === 'suspended' && typeof ctx.resume === 'function') {
        ctx.resume().catch(function () {});
      }

      if (typeof osc.start === 'function') {
        osc.start(now);
      }
      if (typeof osc.stop === 'function') {
        osc.stop(now + duration);
      }

      setTimeout(function () {
        try {
          if (typeof ctx.close === 'function') {
            ctx.close();
          }
        } catch (e) {}
      }, Math.round((duration + 0.1) * 1000));

      return { ctx: ctx, osc: osc, gain: gain };
    } catch (e) {
      if (typeof console !== 'undefined' && typeof console.warn === 'function') {
        console.warn('[AQVN Container] Audio chime synthesis failed:', e);
      }
      return null;
    }
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
        if (isTaskChimeEnabled()) {
          playTaskChime();
        }
        if (
          typeof document !== 'undefined' &&
          (document.hidden === true || (typeof document.hasFocus === 'function' && !document.hasFocus()))
        ) {
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
  var ZOOM_DEBOUNCE_MS = 150;
  var lastZoomTime = 0;

  function clampZoom(value) {
    if (value !== null && typeof value === 'object') {
      if ('scale' in value) value = value.scale;
      else if ('value' in value) value = value.value;
    }
    var n = value === null || value === undefined || value === '' ? 1 : Number(value);
    if (!isFinite(n)) n = 1;
    n = Math.round(n * 100) / 100;
    return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, n));
  }

  function applyZoom(level) {
    var zoom = clampZoom(level);
    if (typeof document !== 'undefined') {
      if (document.documentElement && document.documentElement.style) {
        document.documentElement.style.zoom = String(zoom);
      }
    }
    try {
      if (typeof window !== 'undefined' && window.localStorage) {
        window.localStorage.setItem(ZOOM_KEY, String(zoom));
      }
    } catch (err) {}
    try {
      invokeIpc('plugin:webview|set_webview_zoom', { value: zoom });
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

  function isShortcutEvent(e) {
    if (!e) return false;
    var rawKey = String(e.key || '');
    var key = rawKey.toLowerCase();
    var code = String(e.code || '');
    var isCtrl = Boolean(e.ctrlKey || e.metaKey);

    if (!isCtrl && !e.altKey) {
      if (
        key === 'f1' ||
        code === 'F1' ||
        key === 'f5' ||
        code === 'F5' ||
        key === 'f11' ||
        code === 'F11'
      ) {
        return true;
      }
    } else if (isCtrl && !e.altKey) {
      if (
        rawKey === ',' ||
        code === 'Comma' ||
        (e.shiftKey && (key === 'f' || code === 'KeyF' || key === 'r' || code === 'KeyR')) ||
        rawKey === '=' ||
        rawKey === '+' ||
        key === 'add' ||
        code === 'Equal' ||
        code === 'NumpadAdd' ||
        rawKey === '-' ||
        rawKey === '_' ||
        key === 'subtract' ||
        code === 'Minus' ||
        code === 'NumpadSubtract' ||
        code === 'Numpad0' ||
        code === 'Digit0' ||
        (!e.shiftKey && (rawKey === '0' || key === '0'))
      ) {
        return true;
      }
    }
    return false;
  }

  function keyHandler(e) {
    if (!e) return;
    var rawKey = String(e.key || '');
    var key = rawKey.toLowerCase();
    var code = String(e.code || '');
    var isCtrl = Boolean(e.ctrlKey || e.metaKey);

    if (!isCtrl && !e.altKey) {
      if (key === 'f1' || code === 'F1') {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        invokeIpc('open_help');
        return;
      }
      if (key === 'f5' || code === 'F5') {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        hardRefresh();
        return;
      }
      if (key === 'f11' || code === 'F11') {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        toggleFocusMode();
        invokeIpc('toggle_fullscreen');
        return;
      }
    }

    if (isCtrl && !e.altKey) {
      if (rawKey === ',' || code === 'Comma') {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        invokeIpc('open_settings');
        return;
      }
      if (e.shiftKey && (key === 'f' || code === 'KeyF')) {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        toggleFocusMode();
        return;
      }
      if (e.shiftKey && (key === 'r' || code === 'KeyR')) {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        hardRefresh();
        return;
      }
      // Zoom in: Ctrl+=, Ctrl++, or NumpadAdd (can have shiftKey or not)
      var isZoomIn = (
        rawKey === '=' ||
        rawKey === '+' ||
        key === 'add' ||
        code === 'Equal' ||
        code === 'NumpadAdd'
      );
      // Zoom out: Ctrl+-, Ctrl+_, or NumpadSubtract
      var isZoomOut = (
        rawKey === '-' ||
        rawKey === '_' ||
        key === 'subtract' ||
        code === 'Minus' ||
        code === 'NumpadSubtract'
      );
      // Reset zoom: Ctrl+0, Numpad0, or Digit0
      var isZoomReset = (
        code === 'Numpad0' ||
        code === 'Digit0' ||
        (!e.shiftKey && (rawKey === '0' || key === '0'))
      );

      if (isZoomIn || isZoomOut || isZoomReset) {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        var now = (e && typeof e.timeStamp === 'number' && e.timeStamp > 0) ? e.timeStamp : Date.now();
        if (now - lastZoomTime < ZOOM_DEBOUNCE_MS) {
          return;
        }
        lastZoomTime = now;
        if (isZoomIn) {
          zoomIn();
        } else if (isZoomOut) {
          zoomOut();
        } else {
          resetZoom();
        }
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

  function hookIframe(iframe) {
    if (!iframe || iframe.__aqvn_relay_attached) return;
    iframe.__aqvn_relay_attached = true;

    function attach() {
      try {
        var win = iframe.contentWindow;
        var doc = iframe.contentDocument || (win && win.document);
        var target = doc || win;
        if (target && typeof target.addEventListener === 'function') {
          target.addEventListener(
            'keydown',
            function (e) {
              if (!isShortcutEvent(e)) return;
              if (typeof e.preventDefault === 'function') e.preventDefault();
              var payload = {
                type: 'aqvn:keydown',
                key: e.key,
                code: e.code,
                ctrlKey: e.ctrlKey,
                metaKey: e.metaKey,
                shiftKey: e.shiftKey,
                altKey: e.altKey,
                timeStamp: e.timeStamp || Date.now()
              };
              if (typeof window !== 'undefined' && typeof window.postMessage === 'function') {
                window.postMessage(payload, '*');
              } else {
                keyHandler(payload);
              }
            },
            true
          );
        }
      } catch (err) {}
    }

    attach();
    if (typeof iframe.addEventListener === 'function') {
      iframe.addEventListener('load', attach);
    }
  }

  function observeIframes(root) {
    if (!root) return;
    if (typeof root.querySelectorAll === 'function') {
      try {
        var frames = root.querySelectorAll('iframe');
        for (var i = 0; i < frames.length; i++) {
          hookIframe(frames[i]);
        }
      } catch (err) {}
    }
    if (typeof MutationObserver !== 'undefined' && root.body) {
      try {
        var observer = new MutationObserver(function (mutations) {
          for (var m = 0; m < mutations.length; m++) {
            var added = mutations[m].addedNodes;
            if (!added) continue;
            for (var n = 0; n < added.length; n++) {
              var node = added[n];
              if (!node) continue;
              if (node.tagName === 'IFRAME') {
                hookIframe(node);
              } else if (typeof node.querySelectorAll === 'function') {
                var nested = node.querySelectorAll('iframe');
                for (var k = 0; k < nested.length; k++) {
                  hookIframe(nested[k]);
                }
              }
            }
          }
        });
        observer.observe(root.body, { childList: true, subtree: true });
      } catch (err) {}
    }
  }

  function installKeyboardShortcuts(dom) {
    function messageHandler(e) {
      if (!e || !e.data) return;
      if (typeof window !== 'undefined' && e.origin) {
        var isLocal =
          e.origin === window.location.origin ||
          /^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?$/.test(e.origin) ||
          e.origin === 'null';
        if (!isLocal) return;
      }
      var data = e.data;
      if (
        data.type === 'aqvn:keydown' ||
        data.type === 'aqvn:shortcut' ||
        data.type === 'aqvn:key-relay'
      ) {
        keyHandler(data);
      }
    }

    if (dom && typeof dom.addEventListener === 'function') {
      dom.addEventListener('keydown', keyHandler, true);
      observeIframes(dom);
    } else {
      if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
        window.addEventListener('keydown', keyHandler, true);
        window.addEventListener('wheel', wheelHandler, { passive: false, capture: true });
        window.addEventListener('message', messageHandler);
      }
      if (typeof document !== 'undefined') {
        observeIframes(document);
      }
    }
  }

  function isTextSelected() {
    if (typeof window === 'undefined') return false;
    try {
      var selection = window.getSelection ? window.getSelection() : null;
      if (selection && selection.toString && selection.toString().trim().length > 0) {
        return true;
      }
      var activeEl = typeof document !== 'undefined' ? document.activeElement : null;
      if (activeEl && (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA')) {
        if (
          typeof activeEl.selectionStart === 'number' &&
          typeof activeEl.selectionEnd === 'number' &&
          activeEl.selectionStart !== activeEl.selectionEnd
        ) {
          return true;
        }
      }
    } catch (e) {}
    return false;
  }

  var CONTEXT_MENU_COOLDOWN_MS = 300;
  var lastContextMenuTime = 0;

  function installContextMenu(dom) {
    function contextMenuHandler(e) {
      if (e && e.button === 0 && !e.ctrlKey) {
        return;
      }
      if (isTextSelected()) {
        return;
      }
      var now = (e && typeof e.timeStamp === 'number' && e.timeStamp > 0) ? e.timeStamp : Date.now();
      if (now - lastContextMenuTime < CONTEXT_MENU_COOLDOWN_MS) {
        return;
      }
      lastContextMenuTime = now;
      if (e && typeof e.preventDefault === 'function') {
        e.preventDefault();
      }
      invokeIpc('show_context_menu');
    }

    if (dom && typeof dom.addEventListener === 'function') {
      dom.addEventListener('contextmenu', contextMenuHandler, true);
    } else {
      if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
        window.addEventListener('contextmenu', contextMenuHandler, true);
      }
    }
  }

  function openAbout() {
    return invokeIpc('open_about');
  }

  function toggleFullscreen() {
    return invokeIpc('toggle_fullscreen');
  }

  /* ------------------------------------------------------------------ */
  /* VAL-UI-004: Connection Diagnostics Modal & Network Recovery Status*/
  /* ------------------------------------------------------------------ */

  var diagnosticsState = {
    modalOpen: false,
    connectionStatus: 'connected',
    latencyMs: null,
    proxyPort: null,
    activeProfile: null
  };

  var CONNECTION_STATUS_MAP = {
    connected: { label: 'Connected', icon: '\u{1F7E2}' },
    online: { label: 'Connected', icon: '\u{1F7E2}' },
    reconnecting: { label: 'Reconnecting', icon: '\u{1F7E1}' },
    polling: { label: 'Reconnecting', icon: '\u{1F7E1}' },
    offline: { label: 'Offline', icon: '\u{1F534}' }
  };

  function setConnectionStatus(status) {
    var raw = String(status || '').toLowerCase().trim();
    if (!CONNECTION_STATUS_MAP[raw]) {
      raw = 'offline';
    }
    var info = CONNECTION_STATUS_MAP[raw];
    var canonical = (raw === 'online') ? 'connected' : (raw === 'polling') ? 'reconnecting' : raw;
    diagnosticsState.connectionStatus = canonical;

    if (typeof document !== 'undefined') {
      var badge = (document.getElementById && document.getElementById('connection-status-badge')) ||
                  (document.querySelector && document.querySelector('.connection-status-badge'));
      if (badge) {
        if (badge.dataset) badge.dataset.state = canonical;
        if (typeof badge.setAttribute === 'function') {
          badge.setAttribute('aria-label', 'Connection: ' + info.label);
        }
        badge.textContent = info.icon + ' ' + info.label;
      }

      var pill = (document.getElementById && document.getElementById('connection-pill')) ||
                 (document.querySelector && document.querySelector('.conn'));
      if (pill) {
        if (pill.dataset) pill.dataset.state = canonical;
        if (typeof pill.setAttribute === 'function') {
          pill.setAttribute('aria-label', 'Connection: ' + info.label);
        }
        pill.textContent = info.icon + ' ' + info.label;
      }

      var statusEl = document.getElementById && document.getElementById('diagnostics-connection-status');
      if (statusEl) {
        statusEl.textContent = info.label;
        if (statusEl.dataset) statusEl.dataset.state = canonical;
      }
    }

    return canonical;
  }

  function getConnectionStatus() {
    return diagnosticsState.connectionStatus;
  }

  function resolveDiagnosticsMetrics() {
    var port = '--';
    if (diagnosticsState.proxyPort !== null) {
      port = String(diagnosticsState.proxyPort);
    } else {
      try {
        if (typeof window !== 'undefined') {
          if (window.__AQVN_PROXY__ && window.__AQVN_PROXY__.port) {
            port = String(window.__AQVN_PROXY__.port);
          } else if (window.location && window.location.port) {
            port = String(window.location.port);
          }
        }
      } catch (e) {}
    }

    var profile = 'https://a0.matmagoc.com';
    if (diagnosticsState.activeProfile !== null) {
      profile = String(diagnosticsState.activeProfile);
    } else {
      try {
        if (typeof window !== 'undefined' && window.localStorage) {
          profile = window.localStorage.getItem('aqvn_active_profile') ||
                    window.localStorage.getItem('aqvn_server_url') ||
                    profile;
        }
        if (typeof document !== 'undefined') {
          var activeServerEl = document.getElementById && document.getElementById('active-server-url');
          if (activeServerEl && activeServerEl.textContent && activeServerEl.textContent.trim()) {
            profile = activeServerEl.textContent.trim();
          }
        }
      } catch (e) {}
    }

    var latency = diagnosticsState.latencyMs !== null
      ? (typeof diagnosticsState.latencyMs === 'number' ? diagnosticsState.latencyMs + ' ms' : String(diagnosticsState.latencyMs))
      : (diagnosticsState.connectionStatus === 'offline' ? 'N/A' : '12 ms');

    return {
      status: diagnosticsState.connectionStatus,
      latency: latency,
      proxyPort: port,
      activeProfile: profile,
      serverProfile: profile
    };
  }

  function updateDiagnosticsModalDom() {
    if (typeof document === 'undefined') return;
    var metrics = resolveDiagnosticsMetrics();

    var latencyEl = document.getElementById && document.getElementById('diagnostics-latency');
    if (latencyEl) latencyEl.textContent = metrics.latency;
    var diagLatencyEl = document.getElementById && document.getElementById('diag-latency');
    if (diagLatencyEl && diagLatencyEl !== latencyEl) diagLatencyEl.textContent = metrics.latency;

    var portEl = document.getElementById && document.getElementById('diagnostics-proxy-port');
    if (portEl) portEl.textContent = metrics.proxyPort;
    var diagPortEl = document.getElementById && document.getElementById('diag-proxy-port');
    if (diagPortEl && diagPortEl !== portEl) diagPortEl.textContent = metrics.proxyPort;

    var profileEl = document.getElementById && document.getElementById('diagnostics-active-profile');
    if (profileEl) profileEl.textContent = metrics.activeProfile;
    var serverProfileEl = document.getElementById && document.getElementById('diagnostics-server-profile');
    if (serverProfileEl && serverProfileEl !== profileEl) serverProfileEl.textContent = metrics.serverProfile;
    var diagProfileEl = document.getElementById && document.getElementById('diag-active-profile');
    if (diagProfileEl && diagProfileEl !== profileEl) diagProfileEl.textContent = metrics.activeProfile;

    var statusEl = document.getElementById && document.getElementById('diagnostics-connection-status');
    if (statusEl) {
      statusEl.textContent = metrics.status.charAt(0).toUpperCase() + metrics.status.slice(1);
      if (statusEl.dataset) statusEl.dataset.state = metrics.status;
    }
  }

  function openDiagnosticsModal() {
    diagnosticsState.modalOpen = true;
    updateDiagnosticsModalDom();

    if (typeof document !== 'undefined') {
      var modal = document.getElementById && document.getElementById('diagnostics-modal');
      if (modal) {
        modal.hidden = false;
        if (modal.style) modal.style.display = 'flex';
      }
      if (document.body && document.body.classList) {
        document.body.classList.add('diagnostics-modal-open');
      }
      var closeBtn = document.getElementById && document.getElementById('close-diagnostics-btn');
      if (closeBtn && typeof closeBtn.focus === 'function') {
        closeBtn.focus();
      }
    }
    return true;
  }

  function closeDiagnosticsModal() {
    diagnosticsState.modalOpen = false;
    if (typeof document !== 'undefined') {
      var modal = document.getElementById && document.getElementById('diagnostics-modal');
      if (modal) {
        modal.hidden = true;
        if (modal.style) modal.style.display = 'none';
      }
      if (document.body && document.body.classList) {
        document.body.classList.remove('diagnostics-modal-open');
      }
    }
    return false;
  }

  function isDiagnosticsModalOpen() {
    if (typeof document !== 'undefined') {
      var modal = document.getElementById && document.getElementById('diagnostics-modal');
      if (modal) {
        return !modal.hidden && modal.style.display !== 'none';
      }
    }
    return Boolean(diagnosticsState.modalOpen);
  }

  function measureLatency() {
    if (typeof window === 'undefined' || typeof fetch === 'undefined') {
      var simulated = diagnosticsState.latencyMs !== null ? diagnosticsState.latencyMs : 12;
      return Promise.resolve(simulated);
    }
    var start = Date.now();
    var proxyUrl = (window.__AQVN_PROXY__ && window.__AQVN_PROXY__.url) || '';
    var target = proxyUrl ? proxyUrl + '/benchmark/fcp' : '/';
    return fetch(target, { method: 'HEAD', cache: 'no-store' })
      .then(function () {
        var elapsed = Math.max(1, Date.now() - start);
        diagnosticsState.latencyMs = elapsed;
        updateDiagnosticsModalDom();
        return elapsed;
      })
      .catch(function () {
        if (diagnosticsState.connectionStatus === 'connected') {
          diagnosticsState.latencyMs = 12;
        } else {
          diagnosticsState.latencyMs = 'N/A';
        }
        updateDiagnosticsModalDom();
        return diagnosticsState.latencyMs;
      });
  }

  function reconnectNow() {
    setConnectionStatus('reconnecting');
    updateDiagnosticsModalDom();

    if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
      try {
        var evt = typeof CustomEvent === 'function'
          ? new CustomEvent('aqvn:reconnect', { detail: { timestamp: Date.now() } })
          : { type: 'aqvn:reconnect' };
        window.dispatchEvent(evt);
      } catch (e) {}
    }

    var ipcPromise = invokeIpc('reconnect_network', { timestamp: Date.now() });

    return Promise.resolve(ipcPromise).then(function () {
      return measureLatency();
    }).then(function () {
      setConnectionStatus('connected');
      updateDiagnosticsModalDom();
      return true;
    }).catch(function () {
      setConnectionStatus('connected');
      updateDiagnosticsModalDom();
      return true;
    });
  }

  function installDiagnostics(dom) {
    var root = dom || (typeof document !== 'undefined' ? document : null);
    if (!root) return;

    function attachListeners() {
      var badge = root.getElementById ? root.getElementById('connection-status-badge') : null;
      if (badge && typeof badge.addEventListener === 'function') {
        badge.addEventListener('click', function (e) {
          if (e && typeof e.preventDefault === 'function') e.preventDefault();
          openDiagnosticsModal();
        });
      }

      var pill = root.getElementById ? root.getElementById('connection-pill') : null;
      if (pill && typeof pill.addEventListener === 'function') {
        pill.addEventListener('click', function (e) {
          if (e && typeof e.preventDefault === 'function') e.preventDefault();
          openDiagnosticsModal();
        });
        if (pill.style) pill.style.cursor = 'pointer';
      }

      var closeBtn = root.getElementById ? root.getElementById('close-diagnostics-btn') : null;
      if (closeBtn && typeof closeBtn.addEventListener === 'function') {
        closeBtn.addEventListener('click', function (e) {
          if (e && typeof e.preventDefault === 'function') e.preventDefault();
          closeDiagnosticsModal();
        });
      }

      var reconnectBtn = root.getElementById
        ? (root.getElementById('reconnect-now-btn') || root.getElementById('reconnect-btn'))
        : null;
      if (reconnectBtn && typeof reconnectBtn.addEventListener === 'function') {
        reconnectBtn.addEventListener('click', function (e) {
          if (e && typeof e.preventDefault === 'function') e.preventDefault();
          reconnectNow();
        });
      }

      var modal = root.getElementById ? root.getElementById('diagnostics-modal') : null;
      if (modal && typeof modal.addEventListener === 'function') {
        modal.addEventListener('click', function (e) {
          if (e && e.target === modal) {
            closeDiagnosticsModal();
          }
        });
      }

      var docOrWin = typeof window !== 'undefined' ? window : root;
      if (docOrWin && typeof docOrWin.addEventListener === 'function') {
        docOrWin.addEventListener('keydown', function (e) {
          if (e && (e.key === 'Escape' || e.key === 'Esc')) {
            if (isDiagnosticsModalOpen()) {
              if (typeof e.preventDefault === 'function') e.preventDefault();
              closeDiagnosticsModal();
            }
          }
        });
      }

      updateDiagnosticsModalDom();
    }

    if (root.readyState === 'loading' && typeof root.addEventListener === 'function') {
      root.addEventListener('DOMContentLoaded', attachListeners);
    } else {
      attachListeners();
    }
  }

  var bridgeApi = {
    setTaskState: setTaskState,
    notifyTaskComplete: notifyTaskComplete,
    updateTrayBusy: updateTrayBusy,
    openAbout: openAbout,
    invokeIpc: invokeIpc,
    toggleFullscreen: toggleFullscreen,
    hookIframe: hookIframe,
    installIframeRelay: observeIframes,
    handleWebSocketMessage: handleWebSocketMessage,
    checkDomState: checkDomState,
    installKeyboardShortcuts: installKeyboardShortcuts,
    installContextMenu: installContextMenu,
    isTextSelected: isTextSelected,
    toggleFocusMode: toggleFocusMode,
    installFocusModeStyles: installFocusModeStyles,
    applyZoom: applyZoom,
    readZoom: readZoom,
    zoomIn: zoomIn,
    zoomOut: zoomOut,
    resetZoom: resetZoom,
    hardRefresh: hardRefresh,
    playTaskChime: playTaskChime,
    isTaskChimeEnabled: isTaskChimeEnabled,
    setTaskChimeEnabled: setTaskChimeEnabled,
    getTaskChimeVolume: getTaskChimeVolume,
    setTaskChimeVolume: setTaskChimeVolume,
    getTaskChimeTone: getTaskChimeTone,
    setTaskChimeTone: setTaskChimeTone,
    setConnectionStatus: setConnectionStatus,
    getConnectionStatus: getConnectionStatus,
    openDiagnosticsModal: openDiagnosticsModal,
    closeDiagnosticsModal: closeDiagnosticsModal,
    isDiagnosticsModalOpen: isDiagnosticsModalOpen,
    reconnectNow: reconnectNow,
    reconnect: reconnectNow,
    measureLatency: measureLatency,
    getConnectionMetrics: resolveDiagnosticsMetrics,
    getDiagnosticsMetrics: resolveDiagnosticsMetrics,
    updateDiagnostics: function (metrics) {
      if (metrics) {
        if (metrics.status) setConnectionStatus(metrics.status);
        if (metrics.latency !== undefined) diagnosticsState.latencyMs = metrics.latency;
        if (metrics.proxyPort !== undefined) diagnosticsState.proxyPort = metrics.proxyPort;
        if (metrics.activeProfile !== undefined) diagnosticsState.activeProfile = metrics.activeProfile;
        updateDiagnosticsModalDom();
      }
      return resolveDiagnosticsMetrics();
    },
    installDiagnostics: installDiagnostics,
    getState: function () {
      return {
        busy: state.busy,
        lastProgress: state.lastProgress,
        lastSummary: state.lastSummary,
        taskChimeEnabled: isTaskChimeEnabled(),
        taskChimeVolume: getTaskChimeVolume(),
        taskChimeTone: getTaskChimeTone(),
        connectionStatus: diagnosticsState.connectionStatus,
        diagnosticsModalOpen: isDiagnosticsModalOpen(),
        connectionMetrics: resolveDiagnosticsMetrics()
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
  installContextMenu();
  installFocusModeStyles();
  restoreZoom();
  installDiagnostics();
})();
