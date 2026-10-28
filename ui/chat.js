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
    input: 'Input',
    thought: 'Thought',
    thinking: 'Thought',
    reasoning: 'Thought'
  };

  var dom = {};
  var state = {
    base: '',
    csrf: '',
    serverUrl: 'https://a0.matmagoc.com',
    serverProfiles: ['https://a0.matmagoc.com'],
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
    seq: 0,
    sidebarCollapsed: false,
    canvasCollapsed: false,
    focusMode: false
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

  /* Desktop IPC with a browser-mode no-op so static UI tests keep working. */
  function invokeIpc(command, args) {
    try {
      if (
        window.__TAURI_INTERNALS__ &&
        typeof window.__TAURI_INTERNALS__.invoke === 'function'
      ) {
        return window.__TAURI_INTERNALS__.invoke(command, args || {}).catch(
          function () {}
        );
      }
    } catch (err) {
      /* browser mode: no desktop APIs */
    }
    return Promise.resolve();
  }

  /* ------------------------------------------------------------------ */
  /* Safe Markdown rendering                                            */
  /* ------------------------------------------------------------------ */

  function escapeHtml(value) {
    return String(value == null ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  /* Only web and mail links survive: javascript:/data: URLs become text. */
  function safeUrl(value) {
    var raw = String(value == null ? '' : value).trim();
    return /^(https?:\/\/|mailto:)/i.test(raw) ? raw : '';
  }

  function markdownLib() {
    if (typeof marked !== 'undefined' && marked && typeof marked.parse === 'function') {
      return marked;
    }
    if (window.marked && typeof window.marked.parse === 'function') {
      return window.marked;
    }
    return null;
  }

  function highlightLib() {
    if (typeof hljs !== 'undefined' && hljs && typeof hljs.highlight === 'function') {
      return hljs;
    }
    if (window.hljs && typeof window.hljs.highlight === 'function') {
      return window.hljs;
    }
    return null;
  }

  function highlightCode(code, lang) {
    var lib = highlightLib();
    if (!lib) {
      return escapeHtml(code);
    }
    try {
      if (lang && typeof lib.getLanguage === 'function' && lib.getLanguage(lang)) {
        return lib.highlight(code, { language: lang, ignoreIllegals: true }).value;
      }
      return lib.highlightAuto(code).value;
    } catch (err) {
      return escapeHtml(code);
    }
  }

  function renderCodeBlock(token) {
    var code = token && token.text != null ? token.text : '';
    var info = String((token && token.lang) || '').trim().split(/\s+/)[0];
    var lang = /^[a-z0-9+#._-]+$/i.test(info) ? info : '';
    var body = highlightCode(code, lang);
    var label = lang ? escapeHtml(lang) : 'text';
    return (
      '<div class="code-block">' +
      '<div class="code-head">' +
      '<span class="code-lang">' + label + '</span>' +
      '<button type="button" class="code-copy" data-copy-code aria-label="Copy code">Copy</button>' +
      '</div>' +
      '<pre><code class="hljs' + (lang ? ' language-' + escapeHtml(lang) : '') + '">' +
      body +
      '</code></pre>' +
      '</div>'
    );
  }

  function markdownRenderer(lib) {
    if (!lib || typeof lib.Renderer !== 'function') {
      return null;
    }
    var renderer = new lib.Renderer();
    /* Raw HTML in a message is untrusted: show it as text, never execute it. */
    renderer.html = function (token) {
      return escapeHtml(token && token.text != null ? token.text : token);
    };
    renderer.code = renderCodeBlock;
    renderer.link = function (token) {
      var href = safeUrl(token && token.href);
      var label =
        token && token.tokens && this.parser
          ? this.parser.parseInline(token.tokens)
          : escapeHtml(token && token.text);
      if (!href) {
        return label;
      }
      var title = token && token.title ? ' title="' + escapeHtml(token.title) + '"' : '';
      return (
        '<a href="' + escapeHtml(href) + '" rel="noopener noreferrer"' + title + '>' + label + '</a>'
      );
    };
    renderer.image = function (token) {
      var src = safeUrl(token && token.href);
      var alt = escapeHtml(token && token.text);
      if (!src || !/^https?:\/\//i.test(src)) {
        return alt ? '<span class="md-image-alt">' + alt + '</span>' : '';
      }
      return (
        '<img src="' + escapeHtml(src) + '" alt="' + alt +
        '" loading="lazy" referrerpolicy="no-referrer">'
      );
    };
    return renderer;
  }

  function renderMarkdown(text) {
    var source = String(text == null ? '' : text);
    var lib = markdownLib();
    if (!lib) {
      return '<p>' + escapeHtml(source).replace(/\n/g, '<br>') + '</p>';
    }
    try {
      return lib.parse(source, {
        gfm: true,
        async: false,
        renderer: markdownRenderer(lib) || undefined
      });
    } catch (err) {
      return '<p>' + escapeHtml(source).replace(/\n/g, '<br>') + '</p>';
    }
  }

  /* Message text rendered as safe Markdown. Code blocks get a language badge
   * and a 1-click Copy button; thought messages fold into <details>. */
  function renderContent(node, text, type) {
    if (!node) {
      return;
    }
    var html = renderMarkdown(text);
    var kind = String(type == null ? '' : type).toLowerCase();
    if (type === true || kind === 'thought' || kind === 'thinking' || kind === 'reasoning') {
      html =
        '<details class="thought-step">' +
        '<summary>Thought</summary>' +
        '<div class="thought-body">' + html + '</div>' +
        '</details>';
    }
    node.innerHTML = html;
  }

  /* 1-click code copy. The button sits inside .code-block next to <code>. */
  function copyCode(button) {
    var block = button;
    while (block && !(block.classList && block.classList.contains('code-block'))) {
      block = block.parentNode;
    }
    var code = block && block.querySelector ? block.querySelector('code') : null;
    var text = code ? code.textContent : '';
    var clipboard = typeof navigator !== 'undefined' ? navigator.clipboard : null;
    if (!clipboard || typeof clipboard.writeText !== 'function' || !text) {
      return;
    }
    Promise.resolve(clipboard.writeText(text))
      .then(function () {
        button.textContent = 'Copied!';
        button.classList.add('copied');
        window.setTimeout(function () {
          button.textContent = 'Copy';
          button.classList.remove('copied');
        }, 1200);
      })
      .catch(function () {
        /* Clipboard permission denied: leave the label untouched. */
      });
  }

  function onMessagesClick(event) {
    var target = event.target;
    while (target && target !== dom.messages) {
      if (target.classList && target.classList.contains('code-copy')) {
        copyCode(target);
        return;
      }
      target = target.parentNode;
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
    if (response.status >= 400 && response.status !== 401) {
      return false;
    }
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
      throw new Error(
        'Failed to parse JSON response from /api/' + name + ': ' + err.message
      );
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
    row.className = 'msg msg-' + type + (record.queued ? ' msg-queued' : '');
    var label = TYPE_LABELS[type] || type;
    if (type === 'code_exe' || type === 'tool') {
      label = '$ EXECUTE';
    }
    if (record.queued) {
      label += ' · queued (offline)';
    }
    setText(row.firstChild, label);
    renderContent(row.lastChild, record.content || '', record.thought === true ? 'thought' : type);
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
    var wasActive = state.active === true;
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

    if (state.active !== wasActive) {
      invokeIpc('set_tray_busy', { busy: state.active });
      if (wasActive && !state.active && document.hidden === true) {
        notifyComplete();
      }
    }
  }

  /* Native notification for the true -> false progress transition. */
  function notifyComplete() {
    var body = state.progress || '';
    if (!body && state.order.length) {
      var last = state.logs.get(state.order[state.order.length - 1]);
      body = (last && last.content) || '';
    }
    body = body.replace(/\s+/g, ' ').trim().slice(0, 140);
    if (!body) {
      body = 'The assistant finished responding.';
    }
    invokeIpc('notify_task_complete', { title: 'Task complete', body: body });
  }

  /* ------------------------------------------------------------------ */
  /* Offline outbox                                                     */
  /* ------------------------------------------------------------------ */

  var OUTBOX_KEY = 'aqvn_outbox';

  function readOutbox() {
    try {
      var raw = window.localStorage.getItem(OUTBOX_KEY);
      var list = raw ? JSON.parse(raw) : [];
      return Array.isArray(list) ? list : [];
    } catch (err) {
      return [];
    }
  }

  function writeOutbox(list) {
    try {
      if (list && list.length) {
        window.localStorage.setItem(OUTBOX_KEY, JSON.stringify(list));
      } else {
        window.localStorage.removeItem(OUTBOX_KEY);
      }
    } catch (err) {
      /* storage may be unavailable; the message stays in RAM */
    }
  }

  function queueOutbox(text, messageId) {
    var list = readOutbox();
    list.push({
      text: text,
      context: state.context || '',
      message_id: messageId || uuid(),
      queued_at: Date.now()
    });
    writeOutbox(list);
  }

  function markQueuedSent(messageId) {
    var record = messageId ? state.logs.get(messageId) : null;
    if (record) {
      record.queued = false;
      paintRow(messageId);
    }
  }

  /* Replay queued messages in order once a transport is available. A failed
   * send keeps that message and everything after it in storage for the next
   * connect, so nothing is silently dropped. */
  function flushOutbox() {
    if (!state.online && !state.polling) {
      return Promise.resolve(0);
    }
    var list = readOutbox();
    var sent = 0;

    function next(index) {
      if (index >= list.length) {
        writeOutbox([]);
        return Promise.resolve(sent);
      }
      var item = list[index];
      var text = item && typeof item.text === 'string' ? item.text : '';
      if (!text) {
        return next(index + 1);
      }
      return apiCall('message_async', {
        text: text,
        context: item.context || state.context || '',
        message_id: item.message_id || uuid()
      }).then(
        function (result) {
          sent += 1;
          markQueuedSent(item.message_id);
          if (result && result.context && result.context !== state.context) {
            state.context = result.context;
          }
          return next(index + 1);
        },
        function () {
          writeOutbox(list.slice(index));
          return sent;
        }
      );
    }

    return next(0);
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

  /* Three-state LED: green online, amber polling fallback, red offline. */
  var CONNECTION_STATES = {
    online: { icon: '\u{1F7E2}', label: 'Online' },
    polling: { icon: '\u{1F7E1}', label: 'Polling' },
    offline: { icon: '\u{1F534}', label: 'Offline' }
  };

  function setConnection(mode) {
    var key = CONNECTION_STATES[mode] ? mode : 'offline';
    var info = CONNECTION_STATES[key];
    setText(dom.connection, info.icon + ' ' + info.label);
    if (dom.connection) {
      dom.connection.dataset.state = key;
      dom.connection.setAttribute('aria-label', 'Connection: ' + info.label);
    }
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
    setConnection('offline');
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
            var msg = (err && err.message) || String(err || '');
            var isAuth = /unauthor|forbidden|401|403|invalid session/i.test(msg);
            if (state.everConnected && isAuth) {
              sessionExpired();
            } else {
              showChatError(msg);
            }
            callback({});
          });
      }
    });

    socket.on('connect', function () {
      state.everConnected = true;
      state.online = true;
      setConnection('online');
      showChatError('');
      requestState();
      flushOutbox();
    });

    socket.on('disconnect', function (reason) {
      state.online = false;
      setConnection('offline');
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
      setConnection('offline');
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
        var result =
          ack && typeof ack === 'object'
            ? Array.isArray(ack.results)
              ? ack.results[0]
              : ack
            : null;
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
        var code =
          error && typeof error === 'object' ? error.code : result.code || '';
        var text =
          error && typeof error === 'object'
            ? error.error || error.message || code
            : String(error || result.message || code || '');
        var isNoHandlers =
          code === 'NO_HANDLERS' ||
          /no[_\s-]?handlers/i.test(String(code) + ' ' + String(text));
        if (isNoHandlers) {
          startPolling();
          return;
        }
        if (
          /unauthor|forbidden|401|403|csrf|(?:invalid|expired)[_\s-]?session|session[_\s-]?(?:expired|invalid)/i.test(
            String(code) + ' ' + String(text)
          )
        ) {
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
    setConnection('polling');
    var tick = function () {
      if (!state.polling || state.expired) {
        return;
      }
      apiCall('poll', {
        context: state.context || null,
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
            showChatError(err.message);
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
      .catch(function (err) {
        var msg = (err && err.message) || String(err || '');
        var isAuth = /unauthor|forbidden|401|403|invalid session/i.test(msg);
        if (isAuth) {
          sessionExpired();
          return;
        }
        showChatError('Reconnecting after restart…');
        window.setTimeout(function () {
          if (!state.expired) {
            onServerRestart();
          }
        }, 1500);
      });
  }

  /* ------------------------------------------------------------------ */
  /* Layout: 3 columns, splitters, panel toggles                        */
  /* ------------------------------------------------------------------ */

  var SIDEBAR_KEY = 'aqvn_sidebar_collapsed';
  var CANVAS_KEY = 'aqvn_canvas_collapsed';
  var CANVAS_PATH = '/plugins/aqvn_intel_suite/webui/canvas.html';
  var SIDEBAR_MIN = 160;
  var SIDEBAR_MAX = 400;
  var CANVAS_MIN = 280;
  var CANVAS_MAX = 800;
  var SIDEBAR_DEFAULT = 220;
  var CANVAS_DEFAULT = 450;
  var ZOOM_KEY = 'aqvn_zoom';
  var ZOOM_MIN = 0.7;
  var ZOOM_MAX = 2.0;
  var ZOOM_STEP = 0.1;

  function readCollapsed(key) {
    try {
      return window.localStorage.getItem(key) === 'true';
    } catch (err) {
      return false;
    }
  }

  function persistCollapsed(key, collapsed) {
    try {
      window.localStorage.setItem(key, collapsed ? 'true' : 'false');
    } catch (err) {
      /* storage is optional */
    }
  }

  function setPanelCollapsed(panel, button, key, collapsed) {
    if (!panel) {
      return;
    }
    if (collapsed) {
      panel.setAttribute('hidden', '');
    } else {
      panel.removeAttribute('hidden');
    }
    panel.classList.toggle('collapsed', !!collapsed);
    if (button) {
      button.setAttribute('aria-pressed', collapsed ? 'true' : 'false');
    }
    persistCollapsed(key, !!collapsed);
  }

  function setSidebarCollapsed(collapsed) {
    state.sidebarCollapsed = !!collapsed;
    setPanelCollapsed(dom.sidebar, dom.toggleSidebarBtn, SIDEBAR_KEY, state.sidebarCollapsed);
  }

  function setCanvasCollapsed(collapsed) {
    state.canvasCollapsed = !!collapsed;
    setPanelCollapsed(dom.canvasPane, dom.toggleCanvasBtn, CANVAS_KEY, state.canvasCollapsed);
    if (!state.canvasCollapsed) {
      loadCanvas();
    }
  }

  function toggleSidebar() {
    setSidebarCollapsed(!state.sidebarCollapsed);
  }

  function toggleCanvas() {
    setCanvasCollapsed(!state.canvasCollapsed);
  }

  /* The Intel Suite lives behind the bridge proxy, so the iframe points at
   * the proxy origin when one is known. */
  function loadCanvas() {
    if (!dom.canvasFrame) {
      return;
    }
    var url = state.base ? state.base + CANVAS_PATH : CANVAS_PATH;
    if (dom.canvasFrame.getAttribute('src') !== url) {
      dom.canvasFrame.setAttribute('src', url);
    }
  }

  /* ------------------------------------------------------------------ */
  /* UI zoom and hard refresh                                           */
  /* ------------------------------------------------------------------ */

  function clampZoom(value) {
    var n = value === null || value === undefined || value === '' ? 1 : Number(value);
    if (!isFinite(n)) {
      n = 1;
    }
    n = Math.round(n * 100) / 100;
    return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, n));
  }

  function applyZoom(level) {
    var zoom = clampZoom(level);
    document.body.style.zoom = String(zoom);
    try {
      window.localStorage.setItem(ZOOM_KEY, String(zoom));
    } catch (err) {
      /* storage is optional */
    }
    return zoom;
  }

  function readZoom() {
    try {
      return clampZoom(window.localStorage.getItem(ZOOM_KEY));
    } catch (err) {
      return 1;
    }
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

  /* Force the Intel Suite iframe to reload even when the URL is unchanged. */
  function reloadCanvas() {
    if (!dom.canvasFrame) {
      return;
    }
    dom.canvasFrame.removeAttribute('src');
    loadCanvas();
  }

  /* Full client reset: RAM message state, CSRF token, Socket.IO and canvas. */
  async function hardRefresh() {
    clearLogs();
    applyProgress(false, '');
    try {
      await refreshCsrf();
    } catch (err) {
      showChatError((err && err.message) || String(err || 'CSRF refresh failed'));
    }
    connectSocket();
    reloadCanvas();
  }

  function applySidebarWidth(px) {
    var width = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(px)));
    state.sidebarWidth = width;
    if (dom.sidebar) {
      dom.sidebar.style.width = width + 'px';
    }
    return width;
  }

  function applyCanvasWidth(px) {
    var width = Math.min(CANVAS_MAX, Math.max(CANVAS_MIN, Math.round(px)));
    state.canvasWidth = width;
    if (dom.canvasPane) {
      dom.canvasPane.style.width = width + 'px';
    }
    return width;
  }

  /* Horizontal drag resize. `invert` handles the right splitter: dragging it
   * left widens the canvas pane. Arrow keys move the handle by 16px so the
   * separator stays keyboard-operable. */
  function bindSplitter(splitter, applyWidth, invert) {
    if (!splitter) {
      return;
    }
    function step(width) {
      return applyWidth(invert ? -width : width);
    }
    splitter.addEventListener('mousedown', function (event) {
      if (event.button !== 0) {
        return;
      }
      event.preventDefault();
      var startX = event.clientX;
      var startWidth = invert
        ? state.canvasWidth || CANVAS_DEFAULT
        : state.sidebarWidth || SIDEBAR_DEFAULT;
      splitter.classList.add('dragging');
      function onMove(moveEvent) {
        var delta = moveEvent.clientX - startX;
        applyWidth(invert ? startWidth - delta : startWidth + delta);
      }
      function onUp() {
        document.removeEventListener('mousemove', onMove);
        document.removeEventListener('mouseup', onUp);
        splitter.classList.remove('dragging');
      }
      document.addEventListener('mousemove', onMove);
      document.addEventListener('mouseup', onUp);
    });
    splitter.addEventListener('keydown', function (event) {
      if (event.key === 'ArrowLeft') {
        step(-16);
      } else if (event.key === 'ArrowRight') {
        step(16);
      } else {
        return;
      }
      event.preventDefault();
    });
  }

  function setFocusMode(enabled) {
    state.focusMode = !!enabled;
    document.body.classList.toggle('focus-mode', state.focusMode);
    if (state.focusMode && dom.input && typeof dom.input.focus === 'function') {
      dom.input.focus();
    }
  }

  function toggleFocusMode() {
    setFocusMode(!state.focusMode);
  }

  function handleShortcut(event) {
    var key = String(event.key || '').toLowerCase();
    if (key === 'f5' || (event.ctrlKey && event.shiftKey && key === 'r')) {
      hardRefresh();
      event.preventDefault();
      return true;
    }
    if (key === 'f11' || (event.ctrlKey && event.shiftKey && key === 'f')) {
      toggleFocusMode();
      event.preventDefault();
      return true;
    }
    if ((event.ctrlKey || event.metaKey) && !event.altKey) {
      if (key === '=' || key === '+') {
        zoomIn();
      } else if (key === '-' || key === '_') {
        zoomOut();
      } else if (key === '0') {
        resetZoom();
      } else {
        return false;
      }
      event.preventDefault();
      return true;
    }
    if (!event.altKey || event.ctrlKey || event.metaKey) {
      return false;
    }
    if (key === 's') {
      toggleSidebar();
    } else if (key === 'c') {
      toggleCanvas();
    } else {
      return false;
    }
    event.preventDefault();
    return true;
  }


  function renderSidebarServers() {
    if (!dom.sidebarServerSelect) {
      return;
    }
    dom.sidebarServerSelect.innerHTML = '';
    state.serverProfiles.forEach(function (url) {
      var opt = document.createElement('option');
      opt.value = url;
      opt.textContent = url.replace(/^https?:\/\//, '');
      if (url === state.serverUrl) {
        opt.selected = true;
      }
      dom.sidebarServerSelect.appendChild(opt);
    });
  }

  function initLayout() {
    dom.sidebar = byId('sidebar');
    dom.chatPane = byId('chat-pane');
    dom.canvasPane = byId('canvas-pane');
    dom.canvasFrame = byId('canvas-frame');
    dom.splitterLeft = byId('splitter-left');
    dom.splitterRight = byId('splitter-right');
    dom.toggleSidebarBtn = byId('toggle-sidebar-btn');
    dom.toggleCanvasBtn = byId('toggle-canvas-btn');
    dom.closeCanvasBtn = byId('close-canvas-btn');
    dom.sidebarServerSelect = byId('sidebar-server-select');
    dom.hardRefreshBtn = byId('hard-refresh-btn');

    state.sidebarWidth = SIDEBAR_DEFAULT;
    state.canvasWidth = CANVAS_DEFAULT;

    setSidebarCollapsed(readCollapsed(SIDEBAR_KEY));
    setCanvasCollapsed(readCollapsed(CANVAS_KEY));
    if (!state.canvasCollapsed) {
      loadCanvas();
    }

    if (dom.toggleSidebarBtn) {
      dom.toggleSidebarBtn.addEventListener('click', toggleSidebar);
    }
    if (dom.toggleCanvasBtn) {
      dom.toggleCanvasBtn.addEventListener('click', toggleCanvas);
    }
    if (dom.closeCanvasBtn) {
      dom.closeCanvasBtn.addEventListener('click', function () {
        setCanvasCollapsed(true);
      });
    }
    if (dom.sidebarServerSelect) {
      dom.sidebarServerSelect.addEventListener('change', function () {
        if (dom.sidebarServerSelect.value) {
          setActiveServer(dom.sidebarServerSelect.value);
        }
      });
    }

    if (dom.hardRefreshBtn) {
      dom.hardRefreshBtn.addEventListener('click', function () {
        hardRefresh();
      });
    }

    applyZoom(readZoom());
    document.addEventListener('keydown', handleShortcut);
    bindSplitter(dom.splitterLeft, applySidebarWidth, false);
    bindSplitter(dom.splitterRight, applyCanvasWidth, true);
    renderSidebarServers();
  }

  async function sendMessage(event) {
    event.preventDefault();
    if (state.expired) {
      return;
    }
    var text = dom.input.value.trim();
    if (!text) {
      return;
    }

    var offline = !state.online && !state.polling;
    var messageId = uuid();
    upsertLog({
      id: messageId,
      type: 'user',
      heading: '',
      content: text,
      kvps: {},
      timestamp: Date.now() / 1000,
      no: Number.MAX_SAFE_INTEGER,
      queued: offline
    });
    scrollToBottom();
    dom.input.value = '';
    dom.input.focus();

    if (offline) {
      queueOutbox(text, messageId);
      showChatError('Offline - message queued; it will send when the connection returns.');
      return;
    }

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
      if (!state.online && !state.polling) {
        var record = state.logs.get(messageId);
        if (record) {
          record.queued = true;
          paintRow(messageId);
        }
        queueOutbox(text, messageId);
        showChatError('Offline - message queued; it will send when the connection returns.');
        return;
      }
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

  function renderServerProfiles() {
    if (dom.activeServerUrl) {
      setText(dom.activeServerUrl, state.serverUrl);
      dom.activeServerUrl.title = 'Active Server: ' + state.serverUrl;
    }
    if (dom.chatServerUrl) {
      setText(dom.chatServerUrl, state.serverUrl);
      dom.chatServerUrl.title = 'Active Server: ' + state.serverUrl;
    }
    if (dom.serverProfilesList) {
      dom.serverProfilesList.innerHTML = '';
      state.serverProfiles.forEach(function (url) {
        var opt = document.createElement('option');
        opt.value = url;
        dom.serverProfilesList.appendChild(opt);
      });
    }
    if (dom.serverProfileSelect) {
      dom.serverProfileSelect.innerHTML = '<option value="">Profiles&hellip;</option>';
      state.serverProfiles.forEach(function (url) {
        var opt = document.createElement('option');
        opt.value = url;
        opt.textContent = url.replace(/^https?:\/\//, '');
        if (url === state.serverUrl) {
          opt.selected = true;
        }
        dom.serverProfileSelect.appendChild(opt);
      });
    }
    renderSidebarServers();
  }

  async function setActiveServer(url) {
    if (!url || !/^https?:\/\//i.test(url)) return;
    state.serverUrl = url;
    if (state.serverProfiles.indexOf(url) === -1) {
      state.serverProfiles.push(url);
    }
    try {
      if (window.bridge && typeof window.bridge.saveServerUrl === 'function') {
        await window.bridge.saveServerUrl(url);
      } else if (window.bridge && typeof window.bridge.getStore === 'function') {
        var store = await window.bridge.getStore('settings.json');
        await store.set('server_url', url);
        await store.set('server_profiles', state.serverProfiles);
        await store.save();
      }
    } catch (err) {}
    try {
      window.localStorage.setItem('aqvn_server_url', url);
      window.localStorage.setItem('aqvn_server_profiles', JSON.stringify(state.serverProfiles));
    } catch (err) {}
    renderServerProfiles();
  }

  async function onLogin(event) {
    event.preventDefault();
    setText(dom.loginError, '');
    dom.sessionNotice.hidden = true;

    var serverUrl = dom.serverUrl ? dom.serverUrl.value.trim() : '';
    if (serverUrl) {
      if (!/^https?:\/\//i.test(serverUrl)) {
        setText(dom.loginError, 'Server URL must start with http:// or https://');
        return;
      }
      await setActiveServer(serverUrl);
    }

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
    dom.serverUrl = byId('login-server-url');
    dom.serverProfileSelect = byId('server-profile-select');
    dom.serverProfilesList = byId('server-profiles-list');
    dom.activeServerUrl = byId('active-server-url');
    dom.chatServerUrl = byId('chat-server-url');
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
    dom.connection = byId('connection-pill') || byId('connection');

    if (dom.messages) {
      dom.messages.addEventListener('click', onMessagesClick);
    }

    state.base = resolveBase();
    dom.proxyUrl.value = state.base;

    initLayout();

    try {
      dom.username.value = window.localStorage.getItem(USER_KEY) || '';
    } catch (err) {
      dom.username.value = '';
    }

    if (dom.serverUrl) {
      dom.serverUrl.addEventListener('change', function () {
        setActiveServer(dom.serverUrl.value.trim());
      });
    }
    if (dom.serverProfileSelect) {
      dom.serverProfileSelect.addEventListener('change', function () {
        if (this.value) {
          if (dom.serverUrl) dom.serverUrl.value = this.value;
          setActiveServer(this.value);
        }
      });
    }

    async function loadServerConfig() {
      try {
        if (window.bridge && typeof window.bridge.getServerProfiles === 'function') {
          state.serverProfiles = await window.bridge.getServerProfiles();
          state.serverUrl = await window.bridge.getServerUrl();
        } else if (window.bridge && typeof window.bridge.getStore === 'function') {
          var store = await window.bridge.getStore('settings.json');
          var u = await store.get('server_url');
          if (u) state.serverUrl = u;
          var p = await store.get('server_profiles');
          if (Array.isArray(p) && p.length > 0) state.serverProfiles = p;
        }
      } catch (err) {}
      if (dom.serverUrl) dom.serverUrl.value = state.serverUrl;
      renderServerProfiles();
    }
    loadServerConfig();

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

    setConnection('offline');
    applyProgress(false, '');
    showLogin();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();