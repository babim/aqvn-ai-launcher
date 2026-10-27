/* AQVN Workspace chat shell.
 *
 * Talks only to the local bridge proxy (Rust) or, in browser mode, to a
 * stub/configurable origin that exposes the same routes:
 *   POST /login {username,password}   -> {ok:true}
 *   GET  /proxy/csrf                  -> {ok,token,runtime_id}
 *   POST /api/<name>                  -> JSON (X-CSRF-Token header)
 *   WS   /ws                          -> socket.io namespace
 *
 * The proxy owns the session cookies; page JS only ever sees the CSRF token.
 */
(function () {
  'use strict';

  var TITLE = 'AQVN AI Workspace';
  var PROXY_KEY = 'aqvn.bridgeProxy';
  var USER_KEY = 'aqvn.lastUser';
  var TIMEZONE =
    (typeof Intl !== 'undefined' && Intl.DateTimeFormat
      ? Intl.DateTimeFormat().resolvedOptions().timeZone
      : '') || 'UTC';

  var TYPE_LABELS = {
    user: 'You',
    agent: 'Assistant',
    response: 'Assistant',
    tool: 'Tool',
    code_exe: 'Code',
    browser: 'Browser',
    subagent: 'Subagent',
    mcp: 'MCP',
    info: 'Info',
    hint: 'Hint',
    progress: 'Progress',
    warning: 'Warning',
    error: 'Error',
    util: 'Internal',
    input: 'Input'
  };

  var dom = {};
  var state = {
    base: '',
    csrf: '',
    socket: null,
    context: null,
    contexts: [],
    logs: new Map(),
    order: [],
    rows: new Map(),
    online: false,
    active: false,
    progress: '',
    expired: false,
    everConnected: false,
    polling: false,
    pollTimer: null,
    seq: 0
  };

  function byId(id) {
    return document.getElementById(id);
  }

  function uuid() {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID();
    }
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (ch) {
      var rnd = (Math.random() * 16) | 0;
      return (ch === 'x' ? rnd : (rnd & 0x3) | 0x8).toString(16);
    });
  }

  function setText(node, text) {
    if (node) {
      node.textContent = text == null ? '' : String(text);
    }
  }

  function trimBase(value) {
    var raw = String(value == null ? '' : value).trim();
    if (!raw) {
      return '';
    }
    return raw.replace(/\/+$/, '');
  }

  /* ------------------------------------------------------------------ */
  /* Endpoint resolution                                                */
  /* ------------------------------------------------------------------ */

  function resolveBase() {
    if (window.bridge && window.bridge.proxy && window.bridge.proxy.url) {
      return trimBase(window.bridge.proxy.url);
    }
    if (typeof window.__AQVN_PROXY__ === 'object' && window.__AQVN_PROXY__) {
      return trimBase(window.__AQVN_PROXY__.url || '');
    }
    var params = new URLSearchParams(window.location.search);
    var fromQuery = params.get('proxy');
    if (fromQuery) {
      var clean = trimBase(fromQuery);
      try {
        window.localStorage.setItem(PROXY_KEY, clean);
      } catch (err) {
        /* storage may be unavailable; the query value still works */
      }
      return clean;
    }
    try {
      return trimBase(window.localStorage.getItem(PROXY_KEY) || '');
    } catch (err) {
      return '';
    }
  }

  function describeBase(base) {
    return base || window.location.origin;
  }

  /* ------------------------------------------------------------------ */
  /* HTTP helpers                                                       */
  /* ------------------------------------------------------------------ */

  function isLoginPage(response) {
    var type = (response.headers.get('content-type') || '').toLowerCase();
    return type.indexOf('text/html') !== -1;
  }

  async function refreshCsrf() {
    var response = await fetch(state.base + '/proxy/csrf', {
      headers: { Accept: 'application/json' },
      cache: 'no-store'
    });
    if (!response.ok) {
      throw new Error('CSRF bootstrap failed (HTTP ' + response.status + ')');
    }
    var data = await response.json();
    if (!data || !data.token) {
      throw new Error('CSRF bootstrap returned no token');
    }
    state.csrf = String(data.token);
    return state.csrf;
  }

  async function apiCall(name, body, retried) {
    var token = state.csrf;
    if (!token) {
      token = await refreshCsrf();
    }
    var response = await fetch(state.base + '/api/' + name, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-CSRF-Token': token
      },
      body: JSON.stringify(body || {})
    });

    if (response.status === 403 && retried !== true) {
      await refreshCsrf();
      return apiCall(name, body, true);
    }

    if (isLoginPage(response)) {
      sessionExpired();
      throw new Error('Session expired');
    }

    if (!response.ok) {
      var detail = '';
      try {
        detail = (await response.text()).slice(0, 200);
      } catch (err) {
        detail = '';
      }
      throw new Error(
        'POST /api/' + name + ' failed (HTTP ' + response.status + ')' +
          (detail ? ': ' + detail : '')
      );
    }

    try {
      return await response.json();
    } catch (err) {
      sessionExpired();
      throw new Error('Session expired');
    }
  }

  /* ------------------------------------------------------------------ */
  /* Rendering                                                          */
  /* ------------------------------------------------------------------ */

  function logKey(record) {
    return record.id || 'no-' + record.no;
  }

  function logNo(record) {
    var value = Number(record.no);
    return Number.isFinite(value) ? value : Number.MAX_SAFE_INTEGER;
  }

  function createRow(id) {
    var article = document.createElement('article');
    article.className = 'msg';
    article.dataset.id = id;
    var head = document.createElement('div');
    head.className = 'msg-head';
    var body = document.createElement('div');
    body.className = 'msg-body';
    article.append(head, body);
    return article;
  }

  function paintRow(id) {
    var record = state.logs.get(id);
    var row = state.rows.get(id);
    if (!record || !row) {
      return;
    }
    var type = record.type || 'info';
    row.className = 'msg msg-' + type;
    setText(row.firstChild, TYPE_LABELS[type] || type);
    setText(row.lastChild, record.content || '');
  }

  function placeRow(id) {
    var record = state.logs.get(id);
    var row = state.rows.get(id);
    if (!record || !row) {
      return;
    }
    var anchor = null;
    var children = dom.messages.children;
    for (var i = 0; i < children.length; i += 1) {
      var other = children[i];
      if (other === row) {
        continue;
      }
      var otherRecord = state.logs.get(other.dataset.id);
      if (otherRecord && logNo(otherRecord) > logNo(record)) {
        anchor = other;
        break;
      }
    }
    dom.messages.insertBefore(row, anchor);
  }

  function scrollToBottom() {
    dom.messages.scrollTop = dom.messages.scrollHeight;
  }

  function upsertLog(record) {
    var id = logKey(record);
    if (state.logs.has(id)) {
      /* Cumulative snapshots replace fields; content is never appended. */
      Object.assign(state.logs.get(id), record);
      paintRow(id);
      return id;
    }
    state.logs.set(id, Object.assign({}, record));
    state.order.push(id);
    var row = createRow(id);
    state.rows.set(id, row);
    paintRow(id);
    placeRow(id);
    return id;
  }

  function mergeLogs(logs) {
    var before = dom.messages.childElementCount;
    for (var i = 0; i < logs.length; i += 1) {
      var record = logs[i];
      if (record && (record.id || record.no != null)) {
        upsertLog(record);
      }
    }
    if (dom.messages.childElementCount !== before) {
      scrollToBottom();
    }
  }

  function clearLogs() {
    state.logs.clear();
    state.order = [];
    state.rows.clear();
    dom.messages.textContent = '';
  }

  function applyProgress(active, text) {
    state.active = active === true;
    state.progress = typeof text === 'string' ? text : '';
    if (state.active && state.progress) {
      setText(dom.status, state.progress);
      dom.status.hidden = false;
    } else {
      setText(dom.status, '');
      dom.status.hidden = true;
    }
    dom.stop.disabled = !state.active;
    document.body.classList.toggle('busy', state.active);
  }

  function renderContexts() {
    dom.contexts.textContent = '';
    state.contexts.forEach(function (context) {
      if (!context || !context.id) {
        return;
      }
      var button = document.createElement('button');
      button.type = 'button';
      button.className = 'ctx';
      button.dataset.id = context.id;
      button.textContent =
        context.name || 'Chat ' + (context.no == null ? '' : context.no);
      if (context.id === state.context) {
        button.classList.add('active');
        button.setAttribute('aria-current', 'true');
      }
      button.addEventListener('click', function () {
        switchContext(context.id);
      });
      dom.contexts.append(button);
    });
  }

  function setConnection(label, tone) {
    setText(dom.connection, label);
    dom.connection.dataset.state = tone || 'offline';
  }

  function showChatError(message) {
    if (!message) {
      dom.chatError.hidden = true;
      setText(dom.chatError, '');
      return;
    }
    setText(dom.chatError, message);
    dom.chatError.hidden = false;
  }

  function showChat() {
    dom.loginView.hidden = true;
    dom.chatView.hidden = false;
    dom.input.focus();
  }

  function showLogin() {
    dom.chatView.hidden = true;
    dom.loginView.hidden = false;
    dom.password.value = '';
    dom.username.focus();
  }

  /* ------------------------------------------------------------------ */
  /* Session expiry                                                     */
  /* ------------------------------------------------------------------ */

  function sessionExpired() {
    if (state.expired) {
      return;
    }
    state.expired = true;
    stopPolling();
    disconnectSocket();
    applyProgress(false, '');
    showLogin();
    dom.sessionNotice.hidden = false;
    setText(dom.sessionNotice, 'Session expired — sign in again to continue.');
    showChatError('');
  }

  /* ------------------------------------------------------------------ */
  /* Socket.IO                                                          */
  /* ------------------------------------------------------------------ */

  function disconnectSocket() {
    if (state.socket) {
      state.socket.removeAllListeners();
      state.socket.disconnect();
      state.socket = null;
    }
    state.online = false;
    setConnection('Offline', 'offline');
  }

  function connectSocket() {
    disconnectSocket();

    var socket = io(state.base + '/ws', {
      autoConnect: false,
      reconnection: true,
      reconnectionDelay: 700,
      reconnectionDelayMax: 4000,
      randomizationFactor: 0.3,
      timeout: 20000,
      transports: ['websocket', 'polling'],
      auth: function (callback) {
        refreshCsrf()
          .then(function (token) {
            callback({ csrf_token: token, handlers: ['ws_webui'] });
          })
          .catch(function (err) {
            if (state.everConnected) {
              sessionExpired();
            } else {
              showChatError(err.message);
            }
            callback({});
          });
      }
    });

    socket.on('connect', function () {
      state.everConnected = true;
      state.online = true;
      setConnection('Connected', 'online');
      showChatError('');
      requestState();
    });

    socket.on('disconnect', function (reason) {
      state.online = false;
      setConnection('Reconnecting…', 'offline');
      applyProgress(false, '');
      /* socket.io does not auto-retry a server-initiated disconnect; recover
       * without a page reload (real servers emit one on restart). */
      if (reason === 'io server disconnect' && !state.expired) {
        window.setTimeout(function () {
          if (state.socket === socket) {
            socket.connect();
          }
        }, 500);
      }
    });

    socket.on('connect_error', function (error) {
      state.online = false;
      var message = (error && error.message) || 'connection failed';
      var authFailure = /unauthor|forbidden|403|invalid session/i.test(message);
      if (authFailure && state.everConnected) {
        sessionExpired();
        return;
      }
      setConnection('Offline', 'offline');
      if (!state.expired) {
        showChatError('Connection problem: ' + message);
      }
    });

    socket.on('state_push', onStatePush);
    socket.on('server_restart', onServerRestart);

    state.socket = socket;
    socket.connect();
  }

  function requestState() {
    if (!state.socket || !state.socket.connected) {
      return;
    }
    var correlationId = 'request-' + uuid();
    state.seq += 1;
    state.socket.emit(
      'state_request',
      {
        context: state.context,
        log_from: 0,
        notifications_from: 0,
        timezone: TIMEZONE,
        collections_delta: true,
        correlationId: correlationId
      },
      function (ack) {
        var result = ack && Array.isArray(ack.results) ? ack.results[0] : null;
        if (!result) {
          showChatError('state_request returned no result');
          startPolling();
          return;
        }
        if (result.ok === true) {
          stopPolling();
          return;
        }
        var error = result.error;
        var code = error && typeof error === 'object' ? error.code : '';
        var text =
          error && typeof error === 'object' ? error.error || code : String(error || '');
        if (/unauthor|forbidden|session|csrf/i.test(String(code) + ' ' + String(text))) {
          sessionExpired();
          return;
        }
        /* Server accepted the socket but did not activate the state handler;
         * fall back to polling so the conversation still renders. */
        startPolling();
      }
    );
  }

  function onStatePush(envelope) {
    var snapshot =
      envelope && envelope.data && envelope.data.snapshot
        ? envelope.data.snapshot
        : envelope && envelope.snapshot;
    if (!snapshot) {
      return;
    }
    applySnapshot(snapshot);
  }

  function applySnapshot(snapshot) {
    if (!state.context && snapshot.context) {
      state.context = snapshot.context;
    }
    if (Array.isArray(snapshot.contexts)) {
      state.contexts = snapshot.contexts;
      renderContexts();
    }
    if (Array.isArray(snapshot.logs)) {
      mergeLogs(snapshot.logs);
    }
    if (
      typeof snapshot.log_progress_active === 'boolean' ||
      typeof snapshot.log_progress === 'string'
    ) {
      applyProgress(snapshot.log_progress_active === true, snapshot.log_progress);
    }
  }

  /* Polling fallback: the server may not activate WebSocket handlers (or the
   * socket may be unavailable). /api/poll returns the same snapshot shape. */
  function startPolling() {
    if (state.polling || state.expired) {
      return;
    }
    state.polling = true;
    setConnection('Polling', 'online');
    var tick = function () {
      if (!state.polling || state.expired) {
        return;
      }
      apiCall('poll', {
        context: state.context || '',
        log_from: 0,
        notifications_from: 0,
        timezone: TIMEZONE
      })
        .then(function (snapshot) {
          if (snapshot && typeof snapshot === 'object') {
            applySnapshot(snapshot);
          }
        })
        .catch(function (err) {
          if (!state.expired) {
            stopPolling();
            showChatError(err.message);
            return;
          }
        });
      state.pollTimer = window.setTimeout(tick, 600);
    };
    state.pollTimer = window.setTimeout(tick, 0);
  }

  function stopPolling() {
    state.polling = false;
    if (state.pollTimer) {
      window.clearTimeout(state.pollTimer);
      state.pollTimer = null;
    }
  }

  function onServerRestart() {
    /* runtime_id rotates on server restart: re-bootstrap the token, then
     * re-handshake without reloading the page. */
    state.everConnected = true;
    refreshCsrf()
      .then(function () {
        if (state.socket && state.socket.connected) {
          requestState();
        } else {
          connectSocket();
        }
      })
      .catch(function () {
        sessionExpired();
      });
  }

  /* ------------------------------------------------------------------ */
  /* Actions                                                            */
  /* ------------------------------------------------------------------ */

  async function sendMessage(event) {
    event.preventDefault();
    if (state.expired) {
      return;
    }
    var text = dom.input.value.trim();
    if (!text) {
      return;
    }

    var messageId = uuid();
    upsertLog({
      id: messageId,
      type: 'user',
      heading: '',
      content: text,
      kvps: {},
      timestamp: Date.now() / 1000,
      no: Number.MAX_SAFE_INTEGER
    });
    scrollToBottom();
    dom.input.value = '';
    dom.input.focus();

    try {
      var result = await apiCall('message_async', {
        text: text,
        context: state.context || '',
        message_id: messageId
      });
      if (result && result.context && result.context !== state.context) {
        state.context = result.context;
        requestState();
      }
    } catch (err) {
      showChatError(err.message);
    }
  }

  async function stopResponse() {
    if (!state.context) {
      return;
    }
    try {
      await apiCall('stop', { context: state.context });
    } catch (err) {
      showChatError(err.message);
    }
  }

  async function newChat() {
    try {
      var result = await apiCall('chat_create', {
        current_context: state.context || ''
      });
      if (result && result.ctxid) {
        state.context = result.ctxid;
      } else {
        state.context = null;
      }
      clearLogs();
      renderContexts();
      applyProgress(false, '');
      requestState();
    } catch (err) {
      showChatError(err.message);
    }
  }

  function switchContext(contextId) {
    if (!contextId || contextId === state.context) {
      return false;
    }
    state.context = contextId;
    clearLogs();
    renderContexts();
    applyProgress(false, '');
    requestState();
    return true;
  }

  async function onLogin(event) {
    event.preventDefault();
    setText(dom.loginError, '');
    dom.sessionNotice.hidden = true;

    var username = dom.username.value.trim();
    var password = dom.password.value;
    if (!username || !password) {
      setText(dom.loginError, 'Username and password are required.');
      return;
    }

    var base = trimBase(dom.proxyUrl.value);
    if (base && !/^https?:\/\//i.test(base)) {
      setText(dom.loginError, 'Bridge proxy URL must start with http:// or https://');
      return;
    }
    state.base = base;
    state.expired = false;
    state.everConnected = false;

    dom.loginSubmit.disabled = true;
    try {
      var response = await fetch(state.base + '/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: username, password: password })
      });
      var data = null;
      try {
        data = await response.json();
      } catch (err) {
        data = null;
      }
      if (!response.ok || !data || data.ok !== true) {
        setText(
          dom.loginError,
          (data && data.error) || 'Sign-in failed (HTTP ' + response.status + ')'
        );
        return;
      }

      try {
        window.localStorage.setItem(PROXY_KEY, state.base);
        window.localStorage.setItem(USER_KEY, username);
      } catch (err) {
        /* storage is optional */
      }

      state.context = null;
      state.contexts = [];
      clearLogs();
      renderContexts();
      applyProgress(false, '');
      stopPolling();
      showChat();
      connectSocket();
    } catch (err) {
      setText(dom.loginError, 'Cannot reach the bridge proxy at ' + describeBase(state.base));
    } finally {
      dom.loginSubmit.disabled = false;
    }
  }

  /* ------------------------------------------------------------------ */
  /* Wiring                                                             */
  /* ------------------------------------------------------------------ */

  function init() {
    document.title = TITLE;

    dom.loginView = byId('login-view');
    dom.chatView = byId('chat-view');
    dom.loginForm = byId('login-form');
    dom.username = byId('login-username');
    dom.password = byId('login-password');
    dom.loginSubmit = byId('login-submit');
    dom.loginError = byId('login-error');
    dom.sessionNotice = byId('session-notice');
    dom.proxyUrl = byId('proxy-url');
    dom.contexts = byId('contexts');
    dom.messages = byId('messages');
    dom.status = byId('status-line');
    dom.chatError = byId('chat-error');
    dom.input = byId('message-input');
    dom.composer = byId('composer');
    dom.send = byId('send-btn');
    dom.stop = byId('stop-btn');
    dom.newChat = byId('new-chat-btn');
    dom.connection = byId('connection');

    state.base = resolveBase();
    dom.proxyUrl.value = state.base;
    try {
      dom.username.value = window.localStorage.getItem(USER_KEY) || '';
    } catch (err) {
      dom.username.value = '';
    }

    dom.loginForm.addEventListener('submit', onLogin);
    dom.composer.addEventListener('submit', sendMessage);
    dom.stop.addEventListener('click', stopResponse);
    dom.newChat.addEventListener('click', newChat);

    dom.input.addEventListener('keydown', function (event) {
      if (event.key === 'Enter' && !event.shiftKey) {
        event.preventDefault();
        dom.composer.requestSubmit();
      }
    });

    dom.proxyUrl.addEventListener('change', function () {
      state.base = trimBase(dom.proxyUrl.value);
      try {
        window.localStorage.setItem(PROXY_KEY, state.base);
      } catch (err) {
        /* storage is optional */
      }
    });

    setConnection('Offline', 'offline');
    applyProgress(false, '');
    showLogin();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();