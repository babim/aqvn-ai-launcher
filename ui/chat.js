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
    focusMode: false,
    searchOpen: false,
    searchQuery: '',
    searchCategory: 'all',
    searchActiveIndex: 0,
    searchMatchesCount: 0,
    sidebarTab: 'sessions',
    workspaceFiles: [],
    terminalOpen: false
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
        breaks: true,
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
        var parsed = new URL(clean, window.location.href);
        if (parsed.hostname === 'localhost' || parsed.hostname === '127.0.0.1' || parsed.hostname === '::1') {
          try {
            window.localStorage.setItem(PROXY_KEY, clean);
          } catch (err) {
            /* storage may be unavailable; the query value still works */
          }
          return clean;
        }
      } catch (err) {
        /* invalid URL syntax */
      }
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

  async function refreshCsrf(retries, delayMs) {
    var maxAttempts = typeof retries === 'number' ? retries + 1 : 3;
    var backoff = typeof delayMs === 'number' ? delayMs : 50;
    var lastError;

    for (var attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        var response = await fetch(state.base + '/proxy/csrf', {
          headers: { Accept: 'application/json' },
          cache: 'no-store'
        });
        if (!response.ok) {
          throw new Error('CSRF bootstrap failed (HTTP ' + response.status + ')');
        }
        var data = await response.json();
        if (!data || !data.token) {
          var errDetail = data && (data.error || data.message);
          throw new Error(
            'CSRF bootstrap returned no token' +
              (errDetail ? ' (' + errDetail + ')' : '')
          );
        }
        state.csrf = String(data.token);
        return state.csrf;
      } catch (err) {
        lastError = err;
        var msg = (err && err.message) || String(err || '');
        var isAuth = /unauthor|forbidden|401|403|invalid session|login_required/i.test(msg);
        if (isAuth || attempt >= maxAttempts) {
          break;
        }
        await new Promise(function (resolve) {
          var waitMs = Math.round(backoff * Math.pow(1.5, attempt - 1));
          if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
            window.setTimeout(resolve, waitMs);
          } else if (typeof setTimeout === 'function') {
            setTimeout(resolve, waitMs);
          } else {
            resolve();
          }
        });
      }
    }
    throw lastError;
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
    row.className =
      'msg msg-' + type +
      (record.queued ? ' msg-queued' : '') +
      (record.failed ? ' msg-failed' : '');
    var label = TYPE_LABELS[type] || type;
    if (type === 'code_exe' || type === 'tool') {
      label = '$ EXECUTE';
    }
    if (record.queued) {
      label += ' · queued (offline)';
    } else if (record.failed) {
      label += ' · not sent';
    }
    setText(row.firstChild, label);
    renderContent(row.lastChild, record.content || '', record.thought === true ? 'thought' : type);
    attachMessageActions(row, record.content || '');
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
    if (!dom.messages) return;
    if (state.userScrolledUp) {
      state.unreadMessageCount = (state.unreadMessageCount || 0) + 1;
      if (dom.scrollBottomBtn) {
        dom.scrollBottomBtn.hidden = false;
        if (dom.unreadCount) dom.unreadCount.textContent = String(state.unreadMessageCount);
      }
      return;
    }
    dom.messages.scrollTop = dom.messages.scrollHeight;
  }

  function upsertLog(record) {
    var id = logKey(record);
    if (state.logs.has(id)) {
      /* Cumulative snapshots replace fields; content is never appended. */
      Object.assign(state.logs.get(id), record);
      paintRow(id);
      if (state.searchOpen) {
        applySearchAndFilter();
      }
      return id;
    }
    state.logs.set(id, Object.assign({}, record));
    state.order.push(id);
    var row = createRow(id);
    state.rows.set(id, row);
    paintRow(id);
    placeRow(id);
    if (state.searchOpen) {
      applySearchAndFilter();
    }
    return id;
  }

  function mergeLogs(logs) {
    if (!Array.isArray(logs) || logs.length === 0) {
      return;
    }
    var before = dom.messages.childElementCount;

    // Filter valid log records
    var validLogs = [];
    for (var i = 0; i < logs.length; i += 1) {
      var r = logs[i];
      if (r && (r.id || r.no != null)) {
        validLogs.push(r);
      }
    }
    if (validLogs.length === 0) {
      return;
    }

    // Sort valid logs by logNo ascending
    validLogs.sort(function (a, b) {
      return logNo(a) - logNo(b);
    });

    var fragment = document.createDocumentFragment ? document.createDocumentFragment() : null;
    var fragmentCount = 0;

    for (var j = 0; j < validLogs.length; j += 1) {
      var record = validLogs[j];
      var id = logKey(record);

      if (state.logs.has(id)) {
        // Update existing log
        Object.assign(state.logs.get(id), record);
        paintRow(id);
      } else {
        // New log record
        state.logs.set(id, Object.assign({}, record));
        state.order.push(id);
        var row = createRow(id);
        state.rows.set(id, row);
        paintRow(id);

        if (fragment) {
          fragment.appendChild(row);
          fragmentCount += 1;
        } else {
          placeRow(id);
        }
      }
    }

    // Keep state.order sorted by logNo
    state.order.sort(function (a, b) {
      var recA = state.logs.get(a);
      var recB = state.logs.get(b);
      return logNo(recA) - logNo(recB);
    });

    if (fragment && fragmentCount > 0) {
      if (before === 0) {
        // Initial load or empty messages container: single batch append
        dom.messages.appendChild(fragment);
      } else {
        // Container already has rows; check if all new rows belong at the bottom
        var lastChild = dom.messages.lastChild;
        var lastRecord = lastChild && lastChild.dataset ? state.logs.get(lastChild.dataset.id) : null;
        var firstNewChild = fragment.firstChild;
        var firstNewRecord = firstNewChild && firstNewChild.dataset ? state.logs.get(firstNewChild.dataset.id) : null;

        if (!lastRecord || !firstNewRecord || logNo(firstNewRecord) >= logNo(lastRecord)) {
          // Monotonically increasing: append directly to bottom
          dom.messages.appendChild(fragment);
        } else {
          // Out of order: unpack fragment and place individually via binary search / placeRow
          while (fragment.firstChild) {
            var node = fragment.firstChild;
            if (fragment.removeChild) {
              fragment.removeChild(node);
            } else if (fragment.children) {
              fragment.children.shift();
            }
            if (node.dataset && node.dataset.id) {
              placeRow(node.dataset.id);
            }
          }
        }
      }
    }

    if (state.searchOpen) {
      applySearchAndFilter();
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
      if (
        wasActive &&
        !state.active &&
        (document.hidden === true || (typeof document.hasFocus === 'function' && !document.hasFocus()))
      ) {
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

  var OUTBOX_MAX_ENTRIES = 50;
  var OUTBOX_MAX_BYTES = 256 * 1024;
  /* The drain in flight, so overlapping flushes cannot resend a batch. */
  var outboxFlush = null;

  function readOutbox() {
    try {
      var raw = window.localStorage.getItem(OUTBOX_KEY);
      var list = raw ? JSON.parse(raw) : [];
      return Array.isArray(list) ? list : [];
    } catch (err) {
      return [];
    }
  }

  /* Returns false when the browser refuses the write so callers can tell the
   * user that a queued message is not actually persisted. */
  function writeOutbox(list) {
    try {
      if (list && list.length) {
        window.localStorage.setItem(OUTBOX_KEY, JSON.stringify(list));
      } else {
        window.localStorage.removeItem(OUTBOX_KEY);
      }
      return true;
    } catch (err) {
      /* storage may be unavailable; the message stays in RAM */
      return false;
    }
  }

  /* Cap the queue by entry count and by total bytes. The oldest entries are
   * dropped first; the caller reports the drop to the user. */
  function queueOutbox(text, messageId) {
    var entry = {
      text: text,
      context: state.context || '',
      message_id: messageId || uuid(),
      queued_at: Date.now()
    };
    if (JSON.stringify([entry]).length > OUTBOX_MAX_BYTES) {
      return { stored: false, dropped: 0, error: 'the message is too large to queue' };
    }
    var list = readOutbox();
    // Deduplicate by message_id
    if (messageId && list.some(function(e) { return e.message_id === messageId; })) {
      return { stored: true, dropped: 0, error: '' };
    }
    list.push(entry);
    var dropped = 0;
    while (list.length > OUTBOX_MAX_ENTRIES) {
      list.shift();
      dropped += 1;
    }
    var payload = JSON.stringify(list);
    while (list.length > 1 && payload.length > OUTBOX_MAX_BYTES) {
      list.shift();
      dropped += 1;
      payload = JSON.stringify(list);
    }
    return { stored: writeOutbox(list), dropped: dropped, error: '' };
  }

  /* Put unclaimed entries back in front of anything queued during the drain. */
  function requeueOutbox(remaining) {
    var current = readOutbox();
    return writeOutbox(remaining.concat(current));
  }

  /* Storage refused the write: the row must stop claiming a queued delivery. */
  function markQueueFailed(messageId, reason) {
    var record = messageId ? state.logs.get(messageId) : null;
    if (record) {
      record.queued = false;
      record.failed = true;
      paintRow(messageId);
    }
    showChatError(
      'Message not queued: ' + (reason || 'browser storage is unavailable') +
        '. Copy the text before closing.'
    );
  }

  /* Queue a message and report the outcome to the user. */
  function queueAndReport(text, messageId) {
    if (text && typeof text === 'object') {
      messageId = text.id;
      text = text.content;
    }
    var queued = queueOutbox(text, messageId);
    if (!queued.stored) {
      markQueueFailed(messageId, queued.error || 'browser storage is unavailable');
      return false;
    }
    if (queued.dropped) {
      showChatError(
        'Offline - message queued. ' + queued.dropped +
          ' older queued message(s) dropped: the outbox limit is reached.'
      );
      return true;
    }
    showChatError('Offline - message queued; it will send when the connection returns.');
    return true;
  }

  function markQueuedSent(messageId) {
    var record = messageId ? state.logs.get(messageId) : null;
    if (record) {
      record.queued = false;
      paintRow(messageId);
    }
  }

  /* Replay queued messages in order once a transport is available. The batch
   * is claimed (removed from storage) before the first send, so a reconnect
   * during the drain cannot replay it. A failed send puts its message and the
   * ones after it back into storage for the next connect. */
  function flushOutbox() {
    if (outboxFlush) {
      /* One drain at a time: an overlapping call joins the running one. */
      return outboxFlush;
    }
    if (!state.online && !state.polling) {
      return Promise.resolve(0);
    }
    var list = readOutbox();
    if (!list.length) {
      return Promise.resolve(0);
    }
    try {
      window.localStorage.setItem('aqvn_outbox_inflight', JSON.stringify(list));
    } catch (e) {}
    if (!writeOutbox([])) {
      showChatError('Outbox unavailable: browser storage cannot be updated.');
      return Promise.resolve(0);
    }
    var sent = 0;

    function next(index) {
      if (index >= list.length) {
        try {
          window.localStorage.removeItem('aqvn_outbox_inflight');
        } catch (e) {}
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
          try {
            window.localStorage.removeItem('aqvn_outbox_inflight');
          } catch (e) {}
          if (!requeueOutbox(list.slice(index))) {
            showChatError(
              'Queued message could not be saved again: browser storage is unavailable.'
            );
          }
          return sent;
        }
      );
    }

    outboxFlush = next(0).then(
      function (count) {
        outboxFlush = null;
        return count;
      },
      function (err) {
        outboxFlush = null;
        showChatError(
          'Outbox flush failed: ' + ((err && err.message) || String(err || 'unknown error'))
        );
        return sent;
      }
    );
    return outboxFlush;
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
    if (typeof window !== 'undefined' && window.__AQVN_CONTAINER__ && typeof window.__AQVN_CONTAINER__.setConnectionStatus === 'function') {
      window.__AQVN_CONTAINER__.setConnectionStatus(key === 'online' ? 'connected' : key);
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
    reloadCanvas();
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
            var isAuth = /unauthor|forbidden|401|403|invalid session|login_required/i.test(msg);
            if (state.everConnected && isAuth) {
              sessionExpired();
            } else {
              showChatError(msg);
            }
            var errorObj = err instanceof Error ? err : new Error(msg);
            if (!errorObj.error) {
              errorObj.error = msg;
            }
            callback(errorObj);
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
    socket.on('pty_output', onPtyOutput);
    socket.on('terminal_output', onPtyOutput);
    socket.on('connector_exec_op_result', onPtyOutput);

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
    if (state.online === true) {
      setConnection('online');
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

  function onPtyOutput(data) {
    if (!data) return;
    var chunk = typeof data === 'string' ? data : (data.output || data.chunk || data.data || '');
    if (chunk) {
      appendTerminalOutput(chunk);
    }
  }

  function setTerminalCollapsed(collapsed) {
    state.terminalOpen = !collapsed;
    if (dom.terminalDrawer) {
      if (collapsed) {
        dom.terminalDrawer.setAttribute('hidden', '');
        dom.terminalDrawer.hidden = true;
      } else {
        dom.terminalDrawer.removeAttribute('hidden');
        dom.terminalDrawer.hidden = false;
        if (dom.terminalInput && typeof dom.terminalInput.focus === 'function') {
          dom.terminalInput.focus();
        }
      }
    }
    if (dom.toggleTerminalBtn) {
      dom.toggleTerminalBtn.setAttribute('aria-pressed', collapsed ? 'false' : 'true');
    }
  }

  function toggleTerminal() {
    setTerminalCollapsed(state.terminalOpen);
  }

  /* ------------------------------------------------------------------ */
  /* ANSI VT100 Parser & Terminal Engine 2.0 (VAL-TERM-001..005)       */
  /* ------------------------------------------------------------------ */

  state.terminalTabs = [{ id: 1, name: 'Shell 1', output: '' }];
  state.activeTerminalTabId = 1;
  state.terminalHistory = [];
  state.terminalHistoryIndex = -1;

  var ANSI_FG_COLORS = {
    30: 'ansi-fg-black',
    31: 'ansi-fg-red',
    32: 'ansi-fg-green',
    33: 'ansi-fg-yellow',
    34: 'ansi-fg-blue',
    35: 'ansi-fg-magenta',
    36: 'ansi-fg-cyan',
    37: 'ansi-fg-white',
    90: 'ansi-fg-bright-black',
    91: 'ansi-fg-bright-red',
    92: 'ansi-fg-bright-green',
    93: 'ansi-fg-bright-yellow',
    94: 'ansi-fg-bright-blue',
    95: 'ansi-fg-bright-magenta',
    96: 'ansi-fg-bright-cyan',
    97: 'ansi-fg-bright-white',
  };

  function parseAnsiToHtml(str) {
    if (!str) return '';
    var ansiRegex = /\x1b\[([0-9;]*)m/g;
    var html = '';
    var lastIndex = 0;
    var currentClasses = [];
    var match;

    while ((match = ansiRegex.exec(str)) !== null) {
      var textChunk = str.slice(lastIndex, match.index);
      if (textChunk) {
        if (currentClasses.length > 0) {
          html += '<span class="' + currentClasses.join(' ') + '">' + escapeHtml(textChunk) + '</span>';
        } else {
          html += escapeHtml(textChunk);
        }
      }
      lastIndex = ansiRegex.lastIndex;

      var codes = match[1] ? match[1].split(';').map(Number) : [0];
      for (var i = 0; i < codes.length; i++) {
        var code = codes[i];
        if (code === 0) {
          currentClasses = [];
        } else if (code === 1) {
          if (!currentClasses.includes('ansi-bold')) currentClasses.push('ansi-bold');
        } else if (code === 2) {
          if (!currentClasses.includes('ansi-dim')) currentClasses.push('ansi-dim');
        } else if (code === 3) {
          if (!currentClasses.includes('ansi-italic')) currentClasses.push('ansi-italic');
        } else if (code === 4) {
          if (!currentClasses.includes('ansi-underline')) currentClasses.push('ansi-underline');
        } else if (ANSI_FG_COLORS[code]) {
          currentClasses = currentClasses.filter(function(c) { return !c.startsWith('ansi-fg-'); });
          currentClasses.push(ANSI_FG_COLORS[code]);
        }
      }
    }

    var remaining = str.slice(lastIndex);
    if (remaining) {
      if (currentClasses.length > 0) {
        html += '<span class="' + currentClasses.join(' ') + '">' + escapeHtml(remaining) + '</span>';
      } else {
        html += escapeHtml(remaining);
      }
    }
    return html;
  }

  function appendTerminalOutput(text) {
    if (!dom.terminalOutput || text == null) return;
    var activeTab = state.terminalTabs.find(function(t) { return t.id === state.activeTerminalTabId; });
    var strText = String(text);
    if (activeTab) {
      activeTab.output += strText;
      var lines = activeTab.output.split('\n');
      if (lines.length > 2000) {
        activeTab.output = lines.slice(-2000).join('\n');
      }
    }
    var html = parseAnsiToHtml(strText);
    if (typeof dom.terminalOutput.insertAdjacentHTML === 'function') {
      dom.terminalOutput.insertAdjacentHTML('beforeend', html);
    } else {
      var cleanText = strText.replace(/\x1b\[[0-9;]*m/g, '');
      dom.terminalOutput.textContent = (dom.terminalOutput.textContent || '') + cleanText;
      if (dom.terminalOutput.innerHTML !== undefined) {
        dom.terminalOutput.innerHTML = (dom.terminalOutput.innerHTML || '') + html;
      }
    }
    dom.terminalOutput.scrollTop = dom.terminalOutput.scrollHeight;
  }

  function clearTerminalOutput() {
    if (!dom.terminalOutput) return;
    dom.terminalOutput.textContent = '';
    if (dom.terminalOutput.innerHTML !== undefined) {
      dom.terminalOutput.innerHTML = '';
    }
    var activeTab = state.terminalTabs.find(function(t) { return t.id === state.activeTerminalTabId; });
    if (activeTab) {
      activeTab.output = '';
    }
  }

  function renderTerminalTabs() {
    if (!dom.terminalTabs) return;
    dom.terminalTabs.innerHTML = '';
    state.terminalTabs.forEach(function(tab) {
      var btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'terminal-tab' + (tab.id === state.activeTerminalTabId ? ' active' : '');
      btn.textContent = tab.name;
      btn.dataset.tabId = tab.id;

      if (state.terminalTabs.length > 1) {
        var closeSpan = document.createElement('span');
        closeSpan.className = 'terminal-tab-close';
        closeSpan.textContent = ' ×';
        closeSpan.title = 'Close tab';
        closeSpan.addEventListener('click', function(e) {
          e.stopPropagation();
          closeTerminalTab(tab.id);
        });
        btn.appendChild(closeSpan);
      }

      btn.addEventListener('click', function() {
        switchTerminalTab(tab.id);
      });
      dom.terminalTabs.appendChild(btn);
    });
  }

  function switchTerminalTab(tabId) {
    state.activeTerminalTabId = tabId;
    renderTerminalTabs();
    var activeTab = state.terminalTabs.find(function(t) { return t.id === tabId; });
    if (dom.terminalOutput) {
      dom.terminalOutput.innerHTML = activeTab ? parseAnsiToHtml(activeTab.output) : '';
      dom.terminalOutput.scrollTop = dom.terminalOutput.scrollHeight;
    }
  }

  function addTerminalTab() {
    var nextId = state.terminalTabs.length > 0
      ? Math.max.apply(null, state.terminalTabs.map(function(t) { return t.id; })) + 1
      : 1;
    var newTab = { id: nextId, name: 'Shell ' + nextId, output: '' };
    state.terminalTabs.push(newTab);
    switchTerminalTab(nextId);
  }

  function closeTerminalTab(tabId) {
    if (state.terminalTabs.length <= 1) return;
    state.terminalTabs = state.terminalTabs.filter(function(t) { return t.id !== tabId; });
    if (state.activeTerminalTabId === tabId) {
      state.activeTerminalTabId = state.terminalTabs[state.terminalTabs.length - 1].id;
    }
    switchTerminalTab(state.activeTerminalTabId);
  }

  function handleTerminalKeydown(e) {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (state.terminalHistory.length === 0) return;
      if (state.terminalHistoryIndex === -1) {
        state.terminalHistoryIndex = state.terminalHistory.length - 1;
      } else if (state.terminalHistoryIndex > 0) {
        state.terminalHistoryIndex--;
      }
      dom.terminalInput.value = state.terminalHistory[state.terminalHistoryIndex] || '';
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (state.terminalHistory.length === 0 || state.terminalHistoryIndex === -1) return;
      if (state.terminalHistoryIndex < state.terminalHistory.length - 1) {
        state.terminalHistoryIndex++;
        dom.terminalInput.value = state.terminalHistory[state.terminalHistoryIndex] || '';
      } else {
        state.terminalHistoryIndex = -1;
        dom.terminalInput.value = '';
      }
    }
  }

  function emitTerminalResize() {
    if (!dom.terminalOutput) return;
    var rect = dom.terminalOutput.getBoundingClientRect();
    var cols = Math.max(20, Math.floor(rect.width / 8));
    var rows = Math.max(5, Math.floor(rect.height / 18));
    try {
      if (state.socket && state.socket.connected) {
        state.socket.emit('connector_exec_resize', { cols: cols, rows: rows });
      }
    } catch (e) {}
  }

  async function sendTerminalCommand() {
    if (!dom.terminalInput) return;
    var cmd = dom.terminalInput.value.trim();
    if (!cmd) return;
    dom.terminalInput.value = '';

    state.terminalHistory.push(cmd);
    state.terminalHistoryIndex = -1;

    appendTerminalOutput('$ ' + cmd + '\n');
    try {
      var res = await apiCall('exec', { command: cmd, code: cmd });
      if (res && res.output) {
        appendTerminalOutput(res.output);
      } else if (res && res.error) {
        appendTerminalOutput('\x1b[31mError: ' + res.error + '\x1b[0m\n');
      }
    } catch (err) {
      appendTerminalOutput('\x1b[31mError: ' + (err.message || String(err)) + '\x1b[0m\n');
    }
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
  /* Agent Activity Feed & Security Approval Gate (VAL-AGENT-001..004)  */
  /* ------------------------------------------------------------------ */

  state.auditTrail = [];
  state.sessionAllowedTools = new Set();
  state.pendingApprovalResolver = null;

  function recordAgentActivity(action, target, status, detail) {
    var entry = {
      id: 'act-' + Date.now() + '-' + Math.floor(Math.random() * 1000),
      timestamp: new Date().toISOString(),
      action: action || 'TOOL',
      target: target || '',
      status: status || 'success',
      detail: detail || ''
    };
    state.auditTrail.push(entry);

    if (dom.activityBadgeCount) {
      dom.activityBadgeCount.textContent = String(state.auditTrail.length);
    }

    if (dom.activityFeedList) {
      var emptyMsg = dom.activityFeedList.querySelector('.activity-empty-msg');
      if (emptyMsg) emptyMsg.remove();

      var item = document.createElement('div');
      item.className = 'activity-item status-' + (status === 'denied' ? 'denied' : (status === 'allowed' ? 'allowed' : 'ok'));
      item.innerHTML =
        '<div class="activity-item-head">' +
          '<span class="activity-item-title">' + escapeHtml(entry.action) + '</span>' +
          '<span class="activity-item-time">' + new Date(entry.timestamp).toLocaleTimeString() + '</span>' +
        '</div>' +
        '<div class="activity-item-detail">' + escapeHtml(entry.target + (entry.detail ? ' — ' + entry.detail : '')) + '</div>';
      dom.activityFeedList.insertBefore(item, dom.activityFeedList.firstChild);
    }
  }

  function toggleActivityDrawer() {
    if (!dom.agentActivityDrawer) return;
    var isHidden = dom.agentActivityDrawer.hidden;
    dom.agentActivityDrawer.hidden = !isHidden;
    if (dom.toggleActivityBtn) {
      dom.toggleActivityBtn.setAttribute('aria-pressed', isHidden ? 'true' : 'false');
    }
  }

  function requestActionApproval(tool, command, riskDesc) {
    var policy = 'supervised';
    try {
      policy = localStorage.getItem('aqvn_security_policy') || 'supervised';
    } catch (e) {}

    if (policy === 'permissive') {
      recordAgentActivity(tool, command, 'allowed', 'Auto-allowed (Permissive mode)');
      return Promise.resolve(true);
    }

    if (state.sessionAllowedTools.has(tool)) {
      recordAgentActivity(tool, command, 'allowed', 'Allowed (Session rule)');
      return Promise.resolve(true);
    }

    if (!dom.agentApprovalModal) {
      recordAgentActivity(tool, command, 'allowed', 'Auto-allowed (No approval UI)');
      return Promise.resolve(true);
    }

    if (dom.approvalToolName) dom.approvalToolName.textContent = tool;
    if (dom.approvalToolCmd) dom.approvalToolCmd.textContent = command;
    if (dom.approvalRiskDesc && riskDesc) dom.approvalRiskDesc.textContent = riskDesc;

    dom.agentApprovalModal.hidden = false;

    return new Promise(function(resolve) {
      state.pendingApprovalResolver = function(allowed, forSession) {
        dom.agentApprovalModal.hidden = true;
        state.pendingApprovalResolver = null;
        if (allowed) {
          if (forSession) {
            state.sessionAllowedTools.add(tool);
          }
          recordAgentActivity(tool, command, 'allowed', forSession ? 'Approved for session' : 'Approved once');
          resolve(true);
        } else {
          recordAgentActivity(tool, command, 'denied', 'Denied by user');
          resolve(false);
        }
      };
    });
  }

  function exportAuditJson() {
    var blob = new Blob([JSON.stringify(state.auditTrail, null, 2)], { type: 'application/json' });
    downloadBlob(blob, 'aqvn-audit-trail-' + new Date().toISOString().slice(0, 10) + '.json');
  }

  function exportAuditCsv() {
    var headers = ['timestamp', 'action', 'target', 'status', 'detail'];
    var rows = state.auditTrail.map(function(e) {
      return [
        JSON.stringify(e.timestamp),
        JSON.stringify(e.action),
        JSON.stringify(e.target),
        JSON.stringify(e.status),
        JSON.stringify(e.detail)
      ].join(',');
    });
    var csv = [headers.join(',')].concat(rows).join('\n');
    var blob = new Blob([csv], { type: 'text/csv' });
    downloadBlob(blob, 'aqvn-audit-trail-' + new Date().toISOString().slice(0, 10) + '.csv');
  }

  /* ------------------------------------------------------------------ */
  /* Modern UI Polish & Gateway Radar (VAL-UI-001..006)                */
  /* ------------------------------------------------------------------ */

  state.userScrolledUp = false;
  state.unreadMessageCount = 0;
  state.latencyHistory = [12, 14, 18, 15, 12, 11, 10, 14, 16, 12];

  function showToast(message, type, duration) {
    if (!dom.toastContainer) return;
    var toast = document.createElement('div');
    toast.className = 'toast toast-' + (type || 'info');
    var icon = 'ℹ️';
    if (type === 'success') icon = '✓';
    else if (type === 'error') icon = '✕';
    else if (type === 'warning') icon = '⚠';

    toast.innerHTML = '<span class="toast-icon">' + icon + '</span><span class="toast-text">' + escapeHtml(message) + '</span>';
    toast.addEventListener('click', function() {
      toast.remove();
    });
    dom.toastContainer.appendChild(toast);

    setTimeout(function() {
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(10px)';
      setTimeout(function() { toast.remove(); }, 300);
    }, duration || 3000);
  }

  function handleMessagesScroll() {
    if (!dom.messages) return;
    var threshold = 100;
    var distanceFromBottom = dom.messages.scrollHeight - dom.messages.scrollTop - dom.messages.clientHeight;
    state.userScrolledUp = distanceFromBottom > threshold;

    if (!state.userScrolledUp) {
      state.unreadMessageCount = 0;
      if (dom.scrollBottomBtn) dom.scrollBottomBtn.hidden = true;
    }
  }

  function scrollToBottom() {
    if (!dom.messages) return;
    dom.messages.scrollTop = dom.messages.scrollHeight;
    state.userScrolledUp = false;
    state.unreadMessageCount = 0;
    if (dom.scrollBottomBtn) dom.scrollBottomBtn.hidden = true;
  }

  function attachMessageActions(row, rawText) {
    if (!row) return;
    var target = row.firstChild || row;
    if (typeof target.appendChild !== 'function') return;
    if (target.querySelector && target.querySelector('.message-action-bar')) return;
    var actionBar = document.createElement('div');
    actionBar.className = 'message-action-bar';

    var copyBtn = document.createElement('button');
    copyBtn.type = 'button';
    copyBtn.className = 'msg-action-btn copy-msg-btn';
    copyBtn.title = 'Copy raw markdown';
    copyBtn.textContent = 'Copy';
    copyBtn.addEventListener('click', function(e) {
      e.stopPropagation();
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(rawText);
        showToast('Message copied to clipboard', 'success', 2000);
      }
    });

    var quoteBtn = document.createElement('button');
    quoteBtn.type = 'button';
    quoteBtn.className = 'msg-action-btn quote-msg-btn';
    quoteBtn.title = 'Quote in composer';
    quoteBtn.textContent = 'Quote';
    quoteBtn.addEventListener('click', function(e) {
      e.stopPropagation();
      if (dom.messageInput) {
        var lines = (rawText || '').split('\n').map(function(l) { return '> ' + l; }).join('\n');
        dom.messageInput.value = lines + '\n\n' + dom.messageInput.value;
        dom.messageInput.focus();
        showToast('Quoted in composer', 'info', 2000);
      }
    });

    actionBar.appendChild(copyBtn);
    actionBar.appendChild(quoteBtn);
    target.appendChild(actionBar);

    // Collapsible code blocks >50 lines
    if (typeof row.querySelectorAll === 'function') {
      var codeBlocks = row.querySelectorAll('pre code');
      codeBlocks.forEach(function(code) {
        var lines = (code.textContent || '').split('\n');
        if (lines.length > 50) {
          var pre = code.parentElement;
          if (pre && pre.style) {
            pre.style.maxHeight = '240px';
            pre.style.overflow = 'hidden';
            var collapseBtn = document.createElement('button');
            collapseBtn.type = 'button';
            collapseBtn.className = 'code-collapse-btn';
            collapseBtn.textContent = 'Expand code (' + lines.length + ' lines) ▼';
            collapseBtn.addEventListener('click', function(e) {
              e.stopPropagation();
              if (pre.style.maxHeight === 'none') {
                pre.style.maxHeight = '240px';
                collapseBtn.textContent = 'Expand code (' + lines.length + ' lines) ▼';
              } else {
                pre.style.maxHeight = 'none';
                collapseBtn.textContent = 'Collapse code ▲';
              }
            });
            if (typeof pre.after === 'function') {
              pre.after(collapseBtn);
            }
          }
        }
      });
    }
  }

  var COMMAND_PALETTE_ACTIONS = [
    { title: 'New Conversation', run: function() { resetChat(); } },
    { title: 'Toggle Terminal Drawer', run: function() { toggleTerminal(); } },
    { title: 'Toggle Sessions Sidebar', run: function() { toggleSidebar(); } },
    { title: 'Toggle Intel Suite Canvas', run: function() { toggleCanvas(); } },
    { title: 'Switch Theme: AQVN Dark', run: function() { applyTheme('aqvn-dark'); } },
    { title: 'Switch Theme: OLED Black', run: function() { applyTheme('oled-black'); } },
    { title: 'Switch Theme: Cyberpunk', run: function() { applyTheme('cyberpunk-terminal'); } },
    { title: 'Switch Theme: Clean Light', run: function() { applyTheme('clean-studio-light'); } },
    { title: 'Clear Terminal Output', run: function() { clearTerminalOutput(); } },
    { title: 'Export Markdown Transcript', run: function() { exportMarkdown(); } },
    { title: 'Export JSON Transcript', run: function() { exportJson(); } },
    { title: 'Show Keyboard Shortcuts', run: function() { openShortcutsModal(); } }
  ];

  function openCommandPalette() {
    if (!dom.commandPaletteModal) return;
    dom.commandPaletteModal.hidden = false;
    if (dom.commandPaletteInput) {
      dom.commandPaletteInput.value = '';
      dom.commandPaletteInput.focus();
    }
    renderCommandPaletteResults('');
  }

  function closeCommandPalette() {
    if (dom.commandPaletteModal) dom.commandPaletteModal.hidden = true;
  }

  state.paletteActiveIndex = 0;

  function renderCommandPaletteResults(query) {
    if (!dom.commandPaletteResults) return;
    dom.commandPaletteResults.innerHTML = '';
    var q = (query || '').toLowerCase().trim();
    var filtered = COMMAND_PALETTE_ACTIONS.filter(function(a) {
      return !q || a.title.toLowerCase().includes(q);
    });
    if (state.paletteActiveIndex >= filtered.length) {
      state.paletteActiveIndex = 0;
    }

    filtered.forEach(function(item, idx) {
      var el = document.createElement('div');
      var isActive = idx === state.paletteActiveIndex;
      el.className = 'palette-item' + (isActive ? ' active' : '');
      el.setAttribute('role', 'option');
      el.setAttribute('aria-selected', isActive ? 'true' : 'false');
      el.id = 'palette-opt-' + idx;
      el.textContent = item.title;
      el.addEventListener('click', function() {
        closeCommandPalette();
        item.run();
      });
      dom.commandPaletteResults.appendChild(el);
    });
  }

  function updatePaletteActiveItem() {
    if (!dom.commandPaletteResults) return;
    var items = dom.commandPaletteResults.querySelectorAll('.palette-item');
    items.forEach(function(item, idx) {
      if (idx === state.paletteActiveIndex) {
        item.classList.add('active');
        item.setAttribute('aria-selected', 'true');
        if (typeof item.scrollIntoView === 'function') {
          item.scrollIntoView({ block: 'nearest' });
        }
      } else {
        item.classList.remove('active');
        item.setAttribute('aria-selected', 'false');
      }
    });
  }

  function openShortcutsModal() {
    if (dom.shortcutsModal) dom.shortcutsModal.hidden = false;
  }

  function closeShortcutsModal() {
    if (dom.shortcutsModal) dom.shortcutsModal.hidden = true;
  }

  function renderServerDropdownList() {
    if (!dom.serverDropdownList) return;
    dom.serverDropdownList.innerHTML = '';
    var profiles = ['https://a0.matmagoc.com'];
    try {
      var raw = localStorage.getItem('aqvn_server_profiles');
      if (raw) {
        var parsed = JSON.parse(raw);
        if (Array.isArray(parsed) && parsed.length > 0) profiles = parsed;
      }
    } catch (e) {}

    var activeUrl = state.base || 'https://a0.matmagoc.com';
    profiles.forEach(function(url) {
      var btn = document.createElement('button');
      btn.type = 'button';
      var isActive = (url === activeUrl || activeUrl.includes(url));
      btn.className = 'server-dropdown-item' + (isActive ? ' active' : '');
      btn.textContent = (isActive ? '✓ ' : '') + url;
      btn.addEventListener('click', function() {
        switchActiveServer(url);
      });
      dom.serverDropdownList.appendChild(btn);
    });
  }

  async function switchActiveServer(url) {
    if (dom.serverProfileDropdown) dom.serverProfileDropdown.hidden = true;
    if (dom.activeServerUrl) dom.activeServerUrl.textContent = url;
    state.base = url;
    try {
      localStorage.setItem('aqvn_server_url', url);
    } catch (e) {}

    showToast('Switched to server: ' + url, 'success');
    connectSocket();
  }

  function updateLatencySparkline(latencyMs) {
    if (typeof latencyMs === 'number' && !isNaN(latencyMs)) {
      state.latencyHistory.push(latencyMs);
      if (state.latencyHistory.length > 20) {
        state.latencyHistory.shift();
      }
    }

    if (dom.diagnosticsQuality) {
      var avg = state.latencyHistory.reduce(function(a, b) { return a + b; }, 0) / (state.latencyHistory.length || 1);
      if (avg < 50) {
        dom.diagnosticsQuality.textContent = 'Excellent (' + Math.round(avg) + ' ms)';
        dom.diagnosticsQuality.style.color = 'var(--ok)';
      } else if (avg < 150) {
        dom.diagnosticsQuality.textContent = 'Fair (' + Math.round(avg) + ' ms)';
        dom.diagnosticsQuality.style.color = '#facc15';
      } else {
        dom.diagnosticsQuality.textContent = 'Poor (' + Math.round(avg) + ' ms)';
        dom.diagnosticsQuality.style.color = 'var(--danger)';
      }
    }

    if (dom.latencySparkline) {
      dom.latencySparkline.innerHTML = '';
      var max = Math.max.apply(null, [100].concat(state.latencyHistory));
      state.latencyHistory.forEach(function(lat) {
        var bar = document.createElement('div');
        bar.className = 'sparkline-bar' + (lat > 150 ? ' poor' : (lat > 50 ? ' fair' : ''));
        var pct = Math.max(10, Math.min(100, Math.round((lat / max) * 100)));
        bar.style.height = pct + '%';
        bar.title = lat + ' ms';
        dom.latencySparkline.appendChild(bar);
      });
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
    if (dom.loginView && !dom.loginView.hidden) {
      return;
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
    if (event.defaultPrevented) return false;
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
      if (key === 'f') {
        openSearch();
        event.preventDefault();
        return true;
      }
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
    if (key === 'escape') {
      if (dom.exportMenu && !dom.exportMenu.hidden) {
        closeExportMenu();
        event.preventDefault();
        return true;
      }
      if (state.searchOpen) {
        closeSearch();
        event.preventDefault();
        return true;
      }
    }
    if (!event.altKey || event.ctrlKey || event.metaKey) {
      return false;
    }
    if (key === 's') {
      toggleSidebar();
    } else if (key === 'c') {
      toggleCanvas();
    } else if (key === 't') {
      toggleTerminal();
    } else {
      return false;
    }
    event.preventDefault();
    return true;
  }


  var THEMES = ['aqvn-dark', 'oled-black', 'cyberpunk-terminal', 'clean-studio-light'];
  var THEME_KEY = 'aqvn_theme';

  function readTheme() {
    try {
      var stored = window.localStorage.getItem(THEME_KEY);
      if (stored && THEMES.indexOf(stored) !== -1) {
        return stored;
      }
    } catch (err) {}
    return THEMES[0];
  }

  function applyTheme(theme) {
    if (THEMES.indexOf(theme) === -1) {
      theme = THEMES[0];
    }
    state.theme = theme;
    if (document.documentElement) {
      document.documentElement.setAttribute('data-theme', theme);
    }
    try {
      window.localStorage.setItem(THEME_KEY, theme);
    } catch (err) {}
    if (window.bridge && typeof window.bridge.getStore === 'function') {
      window.bridge.getStore('settings.json').then(function (store) {
        if (store) {
          store.set('app_theme', theme).then(function () {
            if (typeof store.save === 'function') store.save();
          }).catch(function () {});
        }
      }).catch(function () {});
    }
    return theme;
  }

  function cycleTheme() {
    var cur = state.theme || readTheme();
    var idx = THEMES.indexOf(cur);
    var next = THEMES[(idx + 1) % THEMES.length];
    return applyTheme(next);
  }

  /* ------------------------------------------------------------------ */
  /* In-chat search & filter bar                                        */
  /* ------------------------------------------------------------------ */

  function matchesCategory(record, category) {
    if (!category || category === 'all') return true;
    if (!record) return false;
    var type = String(record.type || '').toLowerCase();
    if (category === 'code_exe') {
      return type === 'code_exe' || type === 'code';
    }
    if (category === 'tool') {
      return type === 'tool';
    }
    if (category === 'thought') {
      return record.thought === true || type === 'thought' || type === 'thinking' || type === 'reasoning';
    }
    if (category === 'error') {
      return type === 'error' || record.failed === true;
    }
    return true;
  }

  function openSearch() {
    state.searchOpen = true;
    if (dom.searchDrawer) {
      dom.searchDrawer.hidden = false;
      dom.searchDrawer.removeAttribute('hidden');
    }
    if (dom.searchToggleBtn) {
      dom.searchToggleBtn.setAttribute('aria-expanded', 'true');
    }
    if (dom.searchInput && typeof dom.searchInput.focus === 'function') {
      dom.searchInput.focus();
      if (typeof dom.searchInput.select === 'function') {
        dom.searchInput.select();
      }
    }
    applySearchAndFilter();
  }

  function closeSearch() {
    state.searchOpen = false;
    if (dom.searchDrawer) {
      dom.searchDrawer.hidden = true;
      dom.searchDrawer.setAttribute('hidden', '');
    }
    if (dom.searchToggleBtn) {
      dom.searchToggleBtn.setAttribute('aria-expanded', 'false');
    }
    state.searchQuery = '';
    state.searchCategory = 'all';
    state.searchActiveIndex = 0;
    state.searchMatchesCount = 0;
    if (dom.searchInput) {
      dom.searchInput.value = '';
    }
    if (dom.searchFilters) {
      var pills = [
        dom.filterAllBtn,
        dom.filterCodeBtn,
        dom.filterToolBtn,
        dom.filterThoughtBtn,
        dom.filterErrorBtn
      ];
      for (var i = 0; i < pills.length; i += 1) {
        var pill = pills[i];
        if (!pill) continue;
        var pillCat = (pill.dataset && pill.dataset.category) || (typeof pill.getAttribute === 'function' ? pill.getAttribute('data-category') : null);
        pill.classList.toggle('active', pillCat === 'all');
      }
    }
    // Restore all message visibility and clear highlights
    state.rows.forEach(function (row, id) {
      row.hidden = false;
      row.removeAttribute('hidden');
      paintRow(id);
    });
    if (dom.searchCount) {
      dom.searchCount.textContent = '0 of 0';
    }
  }

  function toggleSearch() {
    if (state.searchOpen) {
      closeSearch();
    } else {
      openSearch();
    }
  }

  function setSearchCategory(category) {
    state.searchCategory = category || 'all';
    if (dom.searchFilters) {
      var pills = [
        dom.filterAllBtn,
        dom.filterCodeBtn,
        dom.filterToolBtn,
        dom.filterThoughtBtn,
        dom.filterErrorBtn
      ];
      for (var i = 0; i < pills.length; i += 1) {
        var pill = pills[i];
        if (!pill) continue;
        var pillCat = (pill.dataset && pill.dataset.category) || (typeof pill.getAttribute === 'function' ? pill.getAttribute('data-category') : null);
        pill.classList.toggle('active', pillCat === state.searchCategory);
      }
    }
    state.searchActiveIndex = 0;
    applySearchAndFilter();
  }

  function highlightMatchesInHtml(html, query, activeMatchIndex) {
    if (!query) return { html: html, count: 0 };
    var escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    var re = new RegExp('(' + escapedQuery + ')', 'gi');
    var matchCount = 0;
    var parts = html.split(/(<[^>]+>)/g);
    for (var i = 0; i < parts.length; i += 1) {
      if (parts[i].charAt(0) !== '<') {
        parts[i] = parts[i].replace(re, function (matched) {
          matchCount += 1;
          var isActive = matchCount === activeMatchIndex;
          return '<mark class="search-match' + (isActive ? ' active' : '') + '">' + escapeHtml(matched) + '</mark>';
        });
      }
    }
    return { html: parts.join(''), count: matchCount };
  }

  function applySearchAndFilter() {
    var query = (state.searchQuery || '').trim();
    var category = state.searchCategory || 'all';
    var totalMatches = 0;
    var matchTargets = [];

    // First count total matches across visible rows matching category
    state.order.forEach(function (id) {
      var record = state.logs.get(id);
      var row = state.rows.get(id);
      if (!record || !row) return;

      var visible = matchesCategory(record, category);
      if (!visible) {
        row.hidden = true;
        row.setAttribute('hidden', '');
        return;
      }
      row.hidden = false;
      row.removeAttribute('hidden');

      if (query) {
        var text = (record.content || '') + ' ' + (record.heading || '');
        var lowerText = text.toLowerCase();
        var lowerQuery = query.toLowerCase();
        var pos = 0;
        while ((pos = lowerText.indexOf(lowerQuery, pos)) !== -1) {
          totalMatches += 1;
          matchTargets.push({ logId: id, matchNum: totalMatches });
          pos += lowerQuery.length;
        }
      }
    });

    state.searchMatchesCount = totalMatches;
    if (totalMatches === 0) {
      state.searchActiveIndex = 0;
      if (dom.searchCount) {
        dom.searchCount.textContent = query ? '0 of 0' : '0 of 0';
      }
    } else {
      if (state.searchActiveIndex <= 0 || state.searchActiveIndex > totalMatches) {
        state.searchActiveIndex = 1;
      }
      if (dom.searchCount) {
        dom.searchCount.textContent = state.searchActiveIndex + ' of ' + totalMatches;
      }
    }

    // Repaint visible rows with highlights
    var currentMatchCounter = 0;
    state.order.forEach(function (id) {
      var record = state.logs.get(id);
      var row = state.rows.get(id);
      if (!record || !row) return;
      if (row.hidden) return;

      // Base render
      paintRow(id);

      if (query && totalMatches > 0) {
        var bodyNode = row.lastChild;
        if (bodyNode) {
          var originalHtml = bodyNode.innerHTML;
          var escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
          var re = new RegExp('(' + escapedQuery + ')', 'gi');
          var parts = originalHtml.split(/(<[^>]+>)/g);
          for (var i = 0; i < parts.length; i += 1) {
            if (parts[i].charAt(0) !== '<') {
              parts[i] = parts[i].replace(re, function (matched) {
                currentMatchCounter += 1;
                var isActive = currentMatchCounter === state.searchActiveIndex;
                return '<mark class="search-match' + (isActive ? ' active' : '') + '">' + matched + '</mark>';
              });
            }
          }
          bodyNode.innerHTML = parts.join('');
        }
      }
    });

    // Scroll active match row into view
    if (state.searchActiveIndex > 0 && matchTargets[state.searchActiveIndex - 1]) {
      var activeLogId = matchTargets[state.searchActiveIndex - 1].logId;
      var activeRow = state.rows.get(activeLogId);
      if (activeRow && typeof activeRow.scrollIntoView === 'function') {
        activeRow.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }

  function searchNext() {
    if (state.searchMatchesCount <= 0) return;
    state.searchActiveIndex = (state.searchActiveIndex % state.searchMatchesCount) + 1;
    applySearchAndFilter();
  }

  function searchPrev() {
    if (state.searchMatchesCount <= 0) return;
    state.searchActiveIndex = state.searchActiveIndex - 1;
    if (state.searchActiveIndex <= 0) {
      state.searchActiveIndex = state.searchMatchesCount;
    }
    applySearchAndFilter();
  }

  /* ------------------------------------------------------------------ */
  /* Conversation Export & Transcript Copy                              */
  /* ------------------------------------------------------------------ */

  function formatTimestamp(ts) {
    if (!ts) return '';
    try {
      var d = new Date(ts > 1e11 ? ts : ts * 1000);
      return d.toISOString().replace('T', ' ').replace(/\..+/, ' UTC');
    } catch (e) {
      return '';
    }
  }

  function getSortedLogEntries() {
    var entries = [];
    state.logs.forEach(function (rec) {
      entries.push(rec);
    });
    entries.sort(function (a, b) {
      return logNo(a) - logNo(b);
    });
    return entries;
  }

  function exportMarkdown() {
    var entries = getSortedLogEntries();
    var lines = [];
    lines.push('# Conversation Transcript');
    lines.push('');
    lines.push('*Exported: ' + formatTimestamp(Date.now() / 1000) + '*');
    lines.push('');

    for (var i = 0; i < entries.length; i += 1) {
      var rec = entries[i];
      var type = rec.type || 'info';
      var roleName = TYPE_LABELS[type] || type;
      if (type === 'user') roleName = 'You';
      else if (type === 'agent' || type === 'response') roleName = 'Assistant';
      else if (type === 'code_exe') roleName = 'Code Execution';
      else if (type === 'tool') roleName = 'Tool Operation';
      else if (rec.thought === true || type === 'thought' || type === 'thinking' || type === 'reasoning') roleName = 'Thought';

      var ts = formatTimestamp(rec.timestamp);
      var header = '## ' + roleName + (ts ? ' (' + ts + ')' : '');
      lines.push(header);
      lines.push('');

      var content = rec.content || '';
      if (type === 'code_exe') {
        lines.push('```bash');
        lines.push(content);
        lines.push('```');
      } else if (rec.thought === true || type === 'thought' || type === 'thinking' || type === 'reasoning') {
        lines.push('> ' + content.replace(/\n/g, '\n> '));
      } else {
        lines.push(content);
      }
      lines.push('');
    }

    return lines.join('\n');
  }

  function exportJson() {
    var entries = getSortedLogEntries();
    return JSON.stringify(entries, null, 2);
  }

  function copyTranscript() {
    var entries = getSortedLogEntries();
    var lines = [];
    for (var i = 0; i < entries.length; i += 1) {
      var rec = entries[i];
      var type = rec.type || 'info';
      var roleName = TYPE_LABELS[type] || type;
      if (type === 'user') roleName = 'You';
      else if (type === 'agent' || type === 'response') roleName = 'Assistant';
      else if (type === 'code_exe') roleName = 'Code Execution';
      else if (type === 'tool') roleName = 'Tool Operation';
      else if (rec.thought === true || type === 'thought' || type === 'thinking' || type === 'reasoning') roleName = 'Thought';

      var ts = formatTimestamp(rec.timestamp);
      lines.push('[' + roleName + (ts ? ' - ' + ts : '') + ']');
      lines.push(rec.content || '');
      lines.push('');
    }

    var transcriptText = lines.join('\n').trim();
    var clipboard = typeof navigator !== 'undefined' ? navigator.clipboard : null;
    if (!clipboard || typeof clipboard.writeText !== 'function') {
      return;
    }

    Promise.resolve(clipboard.writeText(transcriptText))
      .then(function () {
        if (dom.copyTranscriptBtn) {
          var origText = dom.copyTranscriptBtn.textContent;
          dom.copyTranscriptBtn.textContent = 'Copied!';
          window.setTimeout(function () {
            dom.copyTranscriptBtn.textContent = origText || 'Copy Transcript';
          }, 1500);
        }
      })
      .catch(function () {});
  }

  function triggerDownload(content, filename, mimeType) {
    var blob = new Blob([content], { type: mimeType });
    var url = URL.createObjectURL(blob);
    var a = document.createElement('a');
    a.href = url;
    a.download = filename;
    if (document.body && typeof document.body.appendChild === 'function') {
      document.body.appendChild(a);
    }
    if (typeof a.click === 'function') {
      a.click();
    }
    if (document.body && typeof document.body.removeChild === 'function') {
      try {
        document.body.removeChild(a);
      } catch (e) {}
    }
    window.setTimeout(function () {
      URL.revokeObjectURL(url);
    }, 1000);
  }

  function openExportMenu() {
    if (dom.exportMenu) {
      dom.exportMenu.hidden = false;
      dom.exportMenu.removeAttribute('hidden');
    }
    if (dom.exportToggleBtn) {
      dom.exportToggleBtn.setAttribute('aria-expanded', 'true');
    }
  }

  function closeExportMenu() {
    if (dom.exportMenu) {
      dom.exportMenu.hidden = true;
      dom.exportMenu.setAttribute('hidden', '');
    }
    if (dom.exportToggleBtn) {
      dom.exportToggleBtn.setAttribute('aria-expanded', 'false');
    }
  }

  function toggleExportMenu() {
    if (dom.exportMenu && !dom.exportMenu.hidden) {
      closeExportMenu();
    } else {
      openExportMenu();
    }
  }

  /* ------------------------------------------------------------------ */
  /* Workspace File Browser & Drag-and-Drop Uploader                     */
  /* ------------------------------------------------------------------ */

  function formatFileSize(bytes) {
    if (bytes == null || isNaN(bytes)) return '0 B';
    var num = Number(bytes);
    if (num < 1024) return num + ' B';
    if (num < 1024 * 1024) return (num / 1024).toFixed(1) + ' KB';
    if (num < 1024 * 1024 * 1024) return (num / (1024 * 1024)).toFixed(1) + ' MB';
    return (num / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
  }

  function formatFileTimestamp(ts) {
    if (!ts) return '';
    try {
      var d = new Date(ts > 1e11 ? ts : ts * 1000);
      return d.toISOString().replace('T', ' ').replace(/\..+/, ' UTC');
    } catch (e) {
      return '';
    }
  }

  function switchSidebarTab(tabName) {
    state.sidebarTab = tabName;
    if (dom.tabSessionsBtn && dom.tabWorkspaceBtn) {
      if (tabName === 'workspace') {
        dom.tabSessionsBtn.classList.remove('active');
        dom.tabSessionsBtn.setAttribute('aria-selected', 'false');
        dom.tabWorkspaceBtn.classList.add('active');
        dom.tabWorkspaceBtn.setAttribute('aria-selected', 'true');
        if (dom.sessionsView) {
          dom.sessionsView.hidden = true;
          dom.sessionsView.setAttribute('hidden', '');
        }
        if (dom.workspaceView) {
          dom.workspaceView.hidden = false;
          dom.workspaceView.removeAttribute('hidden');
        }
        loadWorkspaceFiles();
      } else {
        dom.tabWorkspaceBtn.classList.remove('active');
        dom.tabWorkspaceBtn.setAttribute('aria-selected', 'false');
        dom.tabSessionsBtn.classList.add('active');
        dom.tabSessionsBtn.setAttribute('aria-selected', 'true');
        if (dom.workspaceView) {
          dom.workspaceView.hidden = true;
          dom.workspaceView.setAttribute('hidden', '');
        }
        if (dom.sessionsView) {
          dom.sessionsView.hidden = false;
          dom.sessionsView.removeAttribute('hidden');
        }
      }
    }
  }

  state.workspaceCurrentPath = '';
  state.previewingFile = null;

  async function loadWorkspaceFiles(subpath) {
    if (typeof subpath === 'string') {
      state.workspaceCurrentPath = subpath;
    }
    renderBreadcrumbs();
    try {
      var res = await apiCall('files_list', { path: state.workspaceCurrentPath });
      var entries = (res && (res.entries || (res.data && res.data.entries))) || [];
      state.workspaceFiles = entries;
      renderWorkspaceFiles();
    } catch (err) {
      renderWorkspaceFiles();
    }
  }

  function renderBreadcrumbs() {
    if (!dom.workspaceBreadcrumbs) return;
    dom.workspaceBreadcrumbs.innerHTML = '';

    var rootBtn = document.createElement('button');
    rootBtn.type = 'button';
    rootBtn.className = 'breadcrumb-item' + (!state.workspaceCurrentPath ? ' active' : '');
    rootBtn.textContent = '~/Workspace';
    rootBtn.dataset.path = '';
    rootBtn.onclick = function() { loadWorkspaceFiles(''); };
    dom.workspaceBreadcrumbs.appendChild(rootBtn);

    if (state.workspaceCurrentPath) {
      var segments = state.workspaceCurrentPath.split('/').filter(Boolean);
      var accum = '';
      segments.forEach(function(seg, i) {
        accum += (accum ? '/' : '') + seg;
        var sep = document.createElement('span');
        sep.className = 'breadcrumb-sep';
        sep.textContent = '>';
        dom.workspaceBreadcrumbs.appendChild(sep);

        var segBtn = document.createElement('button');
        segBtn.type = 'button';
        var isLast = i === segments.length - 1;
        segBtn.className = 'breadcrumb-item' + (isLast ? ' active' : '');
        segBtn.textContent = seg;
        segBtn.dataset.path = accum;
        if (!isLast) {
          var targetPath = accum;
          segBtn.onclick = function() { loadWorkspaceFiles(targetPath); };
        }
        dom.workspaceBreadcrumbs.appendChild(segBtn);
      });
    }
  }

  function renderWorkspaceFiles() {
    if (!dom.workspaceFileList) return;
    dom.workspaceFileList.innerHTML = '';
    if (!state.workspaceFiles || state.workspaceFiles.length === 0) {
      var emptyEl = document.createElement('div');
      emptyEl.className = 'workspace-empty-msg';
      emptyEl.textContent = 'No files found in workspace';
      dom.workspaceFileList.appendChild(emptyEl);
      return;
    }

    state.workspaceFiles.forEach(function (file) {
      var item = document.createElement('div');
      item.className = 'workspace-file-item ' + (file.is_dir ? 'is-dir' : 'is-file');
      var filePath = file.path || file.name;
      item.dataset.path = filePath;

      var icon = file.is_dir ? '&#128193;' : '&#128196;';
      var parts = [];
      if (!file.is_dir && file.size != null) {
        parts.push(formatFileSize(file.size));
      }
      if (file.modified) {
        parts.push(formatFileTimestamp(file.modified));
      }

      var chevron = file.is_dir ? '<span class="file-chevron">&#8250;</span>' : '';
      var actions = '<div class="file-item-actions">' +
        (!file.is_dir ? '<button type="button" class="file-action-icon preview-btn" title="Preview">&#128065;</button>' : '') +
        '<button type="button" class="file-action-icon delete-btn" title="Delete">&#128465;</button>' +
      '</div>';

      item.innerHTML =
        '<span class="file-icon">' + icon + '</span>' +
        '<div class="file-info">' +
          '<div class="file-name">' + escapeHtml(file.name || file.path) + '</div>' +
          '<div class="file-meta">' + escapeHtml(parts.join(' • ')) + '</div>' +
        '</div>' +
        chevron +
        actions;

      item.addEventListener('click', function(e) {
        if (e.target.closest('.delete-btn')) {
          e.stopPropagation();
          deleteWorkspaceItem(filePath, file.is_dir);
          return;
        }
        if (e.target.closest('.preview-btn')) {
          e.stopPropagation();
          openFilePreview(file);
          return;
        }
        if (file.is_dir) {
          loadWorkspaceFiles(filePath);
        } else {
          openFilePreview(file);
        }
      });

      dom.workspaceFileList.appendChild(item);
    });
  }

  async function openFilePreview(file) {
    if (!dom.filePreviewModal) return;
    state.previewingFile = file;
    var filePath = file.path || file.name;
    if (dom.filePreviewTitle) {
      dom.filePreviewTitle.textContent = file.name || filePath;
    }
    if (dom.filePreviewContent) {
      dom.filePreviewContent.innerHTML = '<p class="loading">Loading content...</p>';
      dom.filePreviewContent.hidden = false;
    }
    if (dom.fileEditorTextarea) {
      dom.fileEditorTextarea.hidden = true;
      dom.fileEditorTextarea.value = '';
    }
    if (dom.filePreviewEditBtn) {
      dom.filePreviewEditBtn.hidden = false;
      dom.filePreviewEditBtn.textContent = 'Edit';
    }
    if (dom.filePreviewSaveBtn) {
      dom.filePreviewSaveBtn.hidden = true;
    }
    dom.filePreviewModal.hidden = false;

    if (isBinaryFile(file)) {
      if (/\.(png|jpe?g|gif|webp|bmp|ico|svg)$/i.test(filePath)) {
        try {
          var res = await apiCall('files_read', { path: filePath, binary: true });
          var b64 = res.data || res.content || '';
          dom.filePreviewContent.innerHTML = '<img src="data:image/png;base64,' + b64 + '" alt="' + escapeHtml(filePath) + '" />';
          if (dom.filePreviewEditBtn) dom.filePreviewEditBtn.hidden = true;
          return;
        } catch (e) {}
      }
      dom.filePreviewContent.innerHTML = '<p class="notice">Binary file (' + formatFileSize(file.size || 0) + '). Preview not available.</p>';
      if (dom.filePreviewEditBtn) dom.filePreviewEditBtn.hidden = true;
      return;
    }

    try {
      var res = await apiCall('files_read', { path: filePath });
      var text = (res && (typeof res.content === 'string' ? res.content : (res.data || ''))) || '';
      state.previewingFile.content = text;

      if (/\.md$/i.test(filePath)) {
        dom.filePreviewContent.innerHTML = renderMarkdown(text);
      } else {
        dom.filePreviewContent.innerHTML = '<pre><code>' + escapeHtml(text) + '</code></pre>';
        if (typeof hljs !== 'undefined' && hljs.highlightElement) {
          var codeEl = dom.filePreviewContent.querySelector('code');
          if (codeEl) hljs.highlightElement(codeEl);
        }
      }
    } catch (err) {
      dom.filePreviewContent.innerHTML = '<p class="error">Failed to load file: ' + escapeHtml(err.message || String(err)) + '</p>';
    }
  }

  function closeFilePreview() {
    if (dom.filePreviewModal) dom.filePreviewModal.hidden = true;
    if (dom.filePreviewContent) dom.filePreviewContent.innerHTML = '';
    state.previewingFile = null;
  }

  function toggleFileEditor() {
    if (!state.previewingFile) return;
    var isEditing = !dom.fileEditorTextarea.hidden;
    if (isEditing) {
      dom.fileEditorTextarea.hidden = true;
      dom.filePreviewContent.hidden = false;
      dom.filePreviewEditBtn.textContent = 'Edit';
      dom.filePreviewSaveBtn.hidden = true;
    } else {
      dom.fileEditorTextarea.value = state.previewingFile.content || '';
      dom.fileEditorTextarea.hidden = false;
      dom.filePreviewContent.hidden = true;
      dom.filePreviewEditBtn.textContent = 'Cancel';
      dom.filePreviewSaveBtn.hidden = false;
      dom.fileEditorTextarea.focus();
    }
  }

  async function saveFileEditor() {
    if (!state.previewingFile) return;
    var filePath = state.previewingFile.path || state.previewingFile.name;
    var newContent = dom.fileEditorTextarea.value;
    try {
      await apiCall('files_write', { path: filePath, content: newContent });
      state.previewingFile.content = newContent;
      toggleFileEditor();
      openFilePreview(state.previewingFile);
      loadWorkspaceFiles(state.workspaceCurrentPath);
    } catch (err) {
      alert('Save failed: ' + (err.message || String(err)));
    }
  }

  async function createWorkspaceFile() {
    var name = prompt('Enter new file name:');
    if (!name) return;
    var targetPath = (state.workspaceCurrentPath ? state.workspaceCurrentPath + '/' : '') + name.trim();
    try {
      await apiCall('files_write', { path: targetPath, content: '' });
      loadWorkspaceFiles(state.workspaceCurrentPath);
    } catch (err) {
      alert('Create file failed: ' + (err.message || String(err)));
    }
  }

  async function createWorkspaceFolder() {
    var name = prompt('Enter new folder name:');
    if (!name) return;
    var targetPath = (state.workspaceCurrentPath ? state.workspaceCurrentPath + '/' : '') + name.trim();
    try {
      await apiCall('files_mkdir', { path: targetPath });
      loadWorkspaceFiles(state.workspaceCurrentPath);
    } catch (err) {
      alert('Create folder failed: ' + (err.message || String(err)));
    }
  }

  async function deleteWorkspaceItem(path, isDir) {
    if (!confirm('Are you sure you want to delete ' + (isDir ? 'folder' : 'file') + ' "' + path + '"?')) {
      return;
    }
    try {
      await apiCall('files_remove', { path: path });
      loadWorkspaceFiles(state.workspaceCurrentPath);
    } catch (err) {
      alert('Delete failed: ' + (err.message || String(err)));
    }
  }

  async function downloadWorkspaceZip() {
    try {
      var res = await apiCall('files_zip', { path: state.workspaceCurrentPath || '.' });
      if (res && res.path) {
        alert('ZIP archive created at: ' + res.path);
      } else {
        alert('ZIP archive created successfully.');
      }
    } catch (err) {
      alert('ZIP download failed: ' + (err.message || String(err)));
    }
  }

  function openDiffViewer(title, diffText) {
    if (!dom.diffViewerModal) return;
    if (dom.diffViewerTitle) {
      dom.diffViewerTitle.textContent = title || 'Visual Diff';
    }
    if (dom.diffViewerContent) {
      dom.diffViewerContent.innerHTML = '';
      var lines = (diffText || '').split('\n');
      lines.forEach(function(line) {
        var span = document.createElement('span');
        if (line.startsWith('+')) {
          span.className = 'diff-line-add';
        } else if (line.startsWith('-')) {
          span.className = 'diff-line-del';
        } else if (line.startsWith('@@')) {
          span.className = 'diff-line-hunk';
        }
        span.textContent = line;
        dom.diffViewerContent.appendChild(span);
        dom.diffViewerContent.appendChild(document.createTextNode('\n'));
      });
    }
    dom.diffViewerModal.hidden = false;
  }

  function closeDiffViewer() {
    if (dom.diffViewerModal) dom.diffViewerModal.hidden = true;
  }

  var BINARY_EXT_RE = /\.(png|jpe?g|gif|webp|bmp|ico|tiff?|avif|heic|heif|pdf|zip|tar|gz|tgz|bz2|xz|7z|rar|wasm|bin|exe|dll|dylib|so|mp3|mp4|wav|ogg|flac|aac|m4a|mov|avi|mkv|webm|iso|dmg|apk|jar|class|woff2?|ttf|otf|eot|doc|docx|xls|xlsx|ppt|pptx)$/i;
  var BINARY_MIME_RE = /^(image\/(?!svg\+xml)|audio\/|video\/|font\/|application\/(pdf|zip|x-zip-compressed|gzip|octet-stream|x-tar|x-7z-compressed|x-rar-compressed|wasm|vnd\.))/i;
  var TEXT_EXT_RE = /\.(txt|md|markdown|json|js|mjs|cjs|ts|tsx|jsx|html?|css|scss|sass|less|xml|svg|yaml|yml|toml|ini|cfg|conf|sh|bash|zsh|bat|cmd|ps1|rs|py|rb|php|java|c|cpp|h|hpp|cs|go|sql|env|log|csv|tsv|properties)$/i;
  var TEXT_MIME_RE = /^(text\/|application\/(json|javascript|x-javascript|typescript|xml|x-yaml|yaml|toml|sql|csv|x-sh|xhtml\+xml|graphql|ld\+json))/i;
  var B64_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

  function isBinaryFile(file) {
    if (!file) return false;
    var name = typeof file === 'string' ? file : (file.name || '');
    var type = (file && typeof file.type === 'string') ? file.type.toLowerCase() : '';

    if (type && BINARY_MIME_RE.test(type)) return true;
    if (type && TEXT_MIME_RE.test(type)) return false;
    if (name && BINARY_EXT_RE.test(name)) return true;
    if (name && TEXT_EXT_RE.test(name)) return false;

    if (file && typeof file.isBinary === 'boolean') return file.isBinary;
    return false;
  }

  function bytesToBase64(bytes) {
    if (!bytes || !bytes.length) return '';
    var btoaFn = (typeof btoa === 'function' ? btoa : (typeof window !== 'undefined' && typeof window.btoa === 'function' ? window.btoa : null));
    if (btoaFn) {
      try {
        var bin = '';
        var chunkSize = 8192;
        for (var i = 0; i < bytes.length; i += chunkSize) {
          bin += String.fromCharCode.apply(null, bytes.subarray(i, Math.min(i + chunkSize, bytes.length)));
        }
        return btoaFn(bin);
      } catch (_) {}
    }
    var res = '';
    var len = bytes.length;
    var idx = 0;
    while (idx < len) {
      var b0 = bytes[idx++];
      var b1 = idx < len ? bytes[idx++] : NaN;
      var b2 = idx < len ? bytes[idx++] : NaN;
      res += B64_CHARS.charAt(b0 >> 2);
      res += B64_CHARS.charAt(((b0 & 3) << 4) | (isNaN(b1) ? 0 : (b1 >> 4)));
      res += isNaN(b1) ? '=' : B64_CHARS.charAt(((b1 & 15) << 2) | (isNaN(b2) ? 0 : (b2 >> 6)));
      res += isNaN(b2) ? '=' : B64_CHARS.charAt(b2 & 63);
    }
    return res;
  }

  async function uploadSingleFile(file) {
    if (!file) return;
    var name = file.name || 'file';
    var isBinary = isBinaryFile(file);
    var content = '';
    var encoding = 'utf-8';

    try {
      if (!isBinary && typeof file.arrayBuffer === 'function' && !TEXT_EXT_RE.test(name) && !(file.type && TEXT_MIME_RE.test(file.type))) {
        var peekBuf = await file.arrayBuffer();
        var peekBytes = new Uint8Array(peekBuf);
        var checkLen = Math.min(peekBytes.length, 8192);
        for (var k = 0; k < checkLen; k += 1) {
          if (peekBytes[k] === 0) {
            isBinary = true;
            break;
          }
        }
        if (isBinary) {
          content = bytesToBase64(peekBytes);
        }
      }

      if (isBinary) {
        encoding = 'base64';
        if (!content) {
          if (typeof file.arrayBuffer === 'function') {
            var buf = await file.arrayBuffer();
            content = bytesToBase64(new Uint8Array(buf));
          } else if (typeof FileReader !== 'undefined') {
            content = await new Promise(function (resolve, reject) {
              var reader = new FileReader();
              reader.onload = function () {
                var res = reader.result;
                if (typeof res === 'string') {
                  var commaIdx = res.indexOf(',');
                  resolve(commaIdx >= 0 ? res.slice(commaIdx + 1) : res);
                } else if (res instanceof ArrayBuffer) {
                  resolve(bytesToBase64(new Uint8Array(res)));
                } else {
                  resolve('');
                }
              };
              reader.onerror = reject;
              if (typeof reader.readAsDataURL === 'function') {
                reader.readAsDataURL(file);
              } else if (typeof reader.readAsArrayBuffer === 'function') {
                reader.readAsArrayBuffer(file);
              } else {
                reader.readAsText(file);
              }
            });
          } else if (typeof file.text === 'function') {
            var str = await file.text();
            var rawBytes = [];
            for (var j = 0; j < str.length; j += 1) {
              rawBytes.push(str.charCodeAt(j) & 0xff);
            }
            content = bytesToBase64(new Uint8Array(rawBytes));
          }
        }
      } else {
        encoding = 'utf-8';
        if (typeof file.text === 'function') {
          content = await file.text();
        } else if (typeof FileReader !== 'undefined') {
          content = await new Promise(function (resolve, reject) {
            var reader = new FileReader();
            reader.onload = function () { resolve(reader.result); };
            reader.onerror = reject;
            reader.readAsText(file);
          });
        } else if (typeof file.arrayBuffer === 'function') {
          var textBuf = await file.arrayBuffer();
          if (typeof TextDecoder !== 'undefined') {
            content = new TextDecoder().decode(textBuf);
          } else {
            content = '';
          }
        }
      }
    } catch (e) {
      content = '';
    }

    var destPath = state.workspaceCurrentPath
      ? (state.workspaceCurrentPath.replace(/\/+$/, '') + '/' + name)
      : name;

    await apiCall('files_write', {
      path: destPath,
      name: name,
      content: content,
      encoding: encoding,
      size: file.size != null ? file.size : (isBinary ? Math.round(content.length * 3 / 4) : content.length)
    });
  }

  async function uploadWorkspaceFiles(files) {
    if (!files || !files.length) return;
    var list = Array.from(files);
    for (var i = 0; i < list.length; i += 1) {
      try {
        await uploadSingleFile(list[i]);
      } catch (err) {
        showChatError('File upload failed: ' + (err.message || String(err)));
      }
    }
    await loadWorkspaceFiles();
  }

  function setupDragAndDropUploader() {
    function handleDrop(e, target) {
      if (e && typeof e.preventDefault === 'function') {
        e.preventDefault();
      }
      if (target) {
        target.classList.remove('dragover');
        target.classList.remove('active');
      }
      var dt = e.dataTransfer;
      if (dt && dt.files && dt.files.length > 0) {
        uploadWorkspaceFiles(dt.files);
      }
    }

    function handleDragOver(e, target) {
      if (e && typeof e.preventDefault === 'function') {
        e.preventDefault();
      }
      if (target) {
        target.classList.add('dragover');
        target.classList.add('active');
      }
    }

    function handleDragLeave(e, target) {
      if (target) {
        target.classList.remove('dragover');
        target.classList.remove('active');
      }
    }

    if (dom.workspaceUploadDropzone) {
      dom.workspaceUploadDropzone.addEventListener('dragover', function (e) {
        handleDragOver(e, dom.workspaceUploadDropzone);
      });
      dom.workspaceUploadDropzone.addEventListener('dragleave', function (e) {
        handleDragLeave(e, dom.workspaceUploadDropzone);
      });
      dom.workspaceUploadDropzone.addEventListener('drop', function (e) {
        handleDrop(e, dom.workspaceUploadDropzone);
      });
      dom.workspaceUploadDropzone.addEventListener('click', function () {
        if (dom.workspaceFileInput) {
          dom.workspaceFileInput.click();
        }
      });
    }

    if (dom.workspaceFileInput) {
      dom.workspaceFileInput.addEventListener('change', function () {
        if (dom.workspaceFileInput.files && dom.workspaceFileInput.files.length > 0) {
          uploadWorkspaceFiles(dom.workspaceFileInput.files);
        }
      });
    }

    if (dom.composer) {
      dom.composer.addEventListener('dragover', function (e) {
        handleDragOver(e, dom.composer);
      });
      dom.composer.addEventListener('dragleave', function (e) {
        handleDragLeave(e, dom.composer);
      });
      dom.composer.addEventListener('drop', function (e) {
        handleDrop(e, dom.composer);
      });
    }
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
    dom.themeToggleBtn = byId('theme-toggle-btn');
    dom.searchToggleBtn = byId('search-toggle-btn');
    dom.searchDrawer = byId('search-drawer');
    dom.searchInput = byId('search-input');
    dom.searchCount = byId('search-count');
    dom.searchPrevBtn = byId('search-prev-btn');
    dom.searchNextBtn = byId('search-next-btn');
    dom.searchCloseBtn = byId('search-close-btn');
    dom.searchFilters = byId('search-filters');
    dom.filterAllBtn = byId('filter-all-btn');
    dom.filterCodeBtn = byId('filter-code-btn');
    dom.filterToolBtn = byId('filter-tool-btn');
    dom.filterThoughtBtn = byId('filter-thought-btn');
    dom.filterErrorBtn = byId('filter-error-btn');
    dom.exportToggleBtn = byId('export-toggle-btn');
    dom.exportMenu = byId('export-menu');
    dom.exportMdBtn = byId('export-md-btn');
    dom.exportJsonBtn = byId('export-json-btn');
    dom.copyTranscriptBtn = byId('copy-transcript-btn');
    dom.tabSessionsBtn = byId('tab-sessions-btn');
    dom.tabWorkspaceBtn = byId('tab-workspace-btn');
    dom.sessionsView = byId('sessions-view');
    dom.workspaceView = byId('workspace-view');
    dom.workspaceFileList = byId('workspace-file-list');
    dom.workspaceBreadcrumbs = byId('workspace-breadcrumbs');
    dom.workspaceNewFileBtn = byId('workspace-new-file-btn');
    dom.workspaceNewFolderBtn = byId('workspace-new-folder-btn');
    dom.workspaceZipBtn = byId('workspace-zip-btn');
    dom.workspaceUploadDropzone = byId('workspace-upload-dropzone');
    dom.workspaceFileInput = byId('workspace-file-input');
    dom.workspaceRefreshBtn = byId('workspace-refresh-btn');
    dom.filePreviewModal = byId('file-preview-modal');
    dom.filePreviewTitle = byId('file-preview-title');
    dom.filePreviewEditBtn = byId('file-preview-edit-btn');
    dom.filePreviewSaveBtn = byId('file-preview-save-btn');
    dom.closeFilePreviewBtn = byId('close-file-preview-btn');
    dom.filePreviewContent = byId('file-preview-content');
    dom.fileEditorTextarea = byId('file-editor-textarea');
    dom.diffViewerModal = byId('diff-viewer-modal');
    dom.diffViewerTitle = byId('diff-viewer-title');
    dom.closeDiffViewerBtn = byId('close-diff-viewer-btn');
    dom.diffViewerContent = byId('diff-viewer-content');
    dom.toggleActivityBtn = byId('toggle-activity-btn');
    dom.activityBadgeCount = byId('activity-badge-count');
    dom.agentActivityDrawer = byId('agent-activity-drawer');
    dom.closeActivityDrawerBtn = byId('close-activity-drawer-btn');
    dom.activityFeedList = byId('activity-feed-list');
    dom.auditExportJsonBtn = byId('audit-export-json-btn');
    dom.auditExportCsvBtn = byId('audit-export-csv-btn');
    dom.agentApprovalModal = byId('agent-approval-modal');
    dom.approvalToolName = byId('approval-tool-name');
    dom.approvalToolCmd = byId('approval-tool-cmd');
    dom.approvalRiskDesc = byId('approval-risk-desc');
    dom.approvalDenyBtn = byId('approval-deny-btn');
    dom.approvalAllowOnceBtn = byId('approval-allow-once-btn');
    dom.approvalAllowSessionBtn = byId('approval-allow-session-btn');
    dom.activeServerBtn = byId('active-server-btn');
    dom.serverProfileDropdown = byId('server-profile-dropdown');
    dom.serverDropdownList = byId('server-dropdown-list');
    dom.scrollBottomBtn = byId('scroll-bottom-btn');
    dom.unreadCount = byId('unread-count');
    dom.toastContainer = byId('toast-container');
    dom.commandPaletteModal = byId('command-palette-modal');
    dom.commandPaletteInput = byId('command-palette-input');
    dom.commandPaletteResults = byId('command-palette-results');
    dom.shortcutsModal = byId('shortcuts-modal');
    dom.closeShortcutsBtn = byId('close-shortcuts-btn');
    dom.diagnosticsQuality = byId('diagnostics-quality');
    dom.latencySparkline = byId('latency-sparkline');
    dom.terminalDrawer = byId('terminal-drawer');
    dom.terminalTabs = byId('terminal-tabs');
    dom.terminalAddTabBtn = byId('terminal-add-tab-btn');
    dom.toggleTerminalBtn = byId('toggle-terminal-btn');
    dom.terminalOutput = byId('terminal-output');
    dom.terminalInput = byId('terminal-input');
    dom.terminalForm = byId('terminal-form');
    dom.terminalSendBtn = byId('terminal-send-btn');
    dom.terminalClearBtn = byId('terminal-clear-btn');
    dom.terminalCloseBtn = byId('terminal-close-btn');

    if (dom.terminalDrawer) {
      dom.terminalDrawer.hidden = true;
    }

    if (dom.exportMenu) {
      dom.exportMenu.hidden = true;
    }

    if (dom.searchDrawer) {
      dom.searchDrawer.hidden = true;
    }

    if (dom.sessionsView) {
      dom.sessionsView.hidden = false;
    }
    if (dom.workspaceView) {
      dom.workspaceView.hidden = true;
    }

    if (dom.tabSessionsBtn) {
      dom.tabSessionsBtn.addEventListener('click', function () {
        switchSidebarTab('sessions');
      });
    }
    if (dom.tabWorkspaceBtn) {
      dom.tabWorkspaceBtn.addEventListener('click', function () {
        switchSidebarTab('workspace');
      });
    }
    if (dom.workspaceRefreshBtn) {
      dom.workspaceRefreshBtn.addEventListener('click', function () {
        loadWorkspaceFiles();
      });
    }
    if (dom.workspaceNewFileBtn) {
      dom.workspaceNewFileBtn.addEventListener('click', createWorkspaceFile);
    }
    if (dom.workspaceNewFolderBtn) {
      dom.workspaceNewFolderBtn.addEventListener('click', createWorkspaceFolder);
    }
    if (dom.workspaceZipBtn) {
      dom.workspaceZipBtn.addEventListener('click', downloadWorkspaceZip);
    }
    if (dom.closeFilePreviewBtn) {
      dom.closeFilePreviewBtn.addEventListener('click', closeFilePreview);
    }
    if (dom.filePreviewEditBtn) {
      dom.filePreviewEditBtn.addEventListener('click', toggleFileEditor);
    }
    if (dom.filePreviewSaveBtn) {
      dom.filePreviewSaveBtn.addEventListener('click', saveFileEditor);
    }
    if (dom.closeDiffViewerBtn) {
      dom.closeDiffViewerBtn.addEventListener('click', closeDiffViewer);
    }
    if (dom.toggleActivityBtn) {
      dom.toggleActivityBtn.addEventListener('click', toggleActivityDrawer);
    }
    if (dom.closeActivityDrawerBtn) {
      dom.closeActivityDrawerBtn.addEventListener('click', toggleActivityDrawer);
    }
    if (dom.auditExportJsonBtn) {
      dom.auditExportJsonBtn.addEventListener('click', exportAuditJson);
    }
    if (dom.auditExportCsvBtn) {
      dom.auditExportCsvBtn.addEventListener('click', exportAuditCsv);
    }
    if (dom.approvalDenyBtn) {
      dom.approvalDenyBtn.addEventListener('click', function() {
        if (state.pendingApprovalResolver) state.pendingApprovalResolver(false, false);
      });
    }
    if (dom.approvalAllowOnceBtn) {
      dom.approvalAllowOnceBtn.addEventListener('click', function() {
        if (state.pendingApprovalResolver) state.pendingApprovalResolver(true, false);
      });
    }
    if (dom.approvalAllowSessionBtn) {
      dom.approvalAllowSessionBtn.addEventListener('click', function() {
        if (state.pendingApprovalResolver) state.pendingApprovalResolver(true, true);
      });
    }
    setupDragAndDropUploader();

    state.sidebarWidth = SIDEBAR_DEFAULT;
    state.canvasWidth = CANVAS_DEFAULT;

    applyTheme(readTheme());

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
    if (dom.themeToggleBtn) {
      dom.themeToggleBtn.addEventListener('click', function () {
        cycleTheme();
      });
    }
    if (dom.exportToggleBtn) {
      dom.exportToggleBtn.addEventListener('click', function (e) {
        if (typeof e.stopPropagation === 'function') e.stopPropagation();
        toggleExportMenu();
      });
    }
    if (dom.exportMdBtn) {
      dom.exportMdBtn.addEventListener('click', function () {
        closeExportMenu();
        var md = exportMarkdown();
        var filename = 'transcript-' + (state.context || 'conversation') + '-' + Date.now() + '.md';
        triggerDownload(md, filename, 'text/markdown');
      });
    }
    if (dom.exportJsonBtn) {
      dom.exportJsonBtn.addEventListener('click', function () {
        closeExportMenu();
        var json = exportJson();
        var filename = 'transcript-' + (state.context || 'conversation') + '-' + Date.now() + '.json';
        triggerDownload(json, filename, 'application/json');
      });
    }
    if (dom.copyTranscriptBtn) {
      dom.copyTranscriptBtn.addEventListener('click', function () {
        copyTranscript();
      });
    }
    if (dom.searchToggleBtn) {
      dom.searchToggleBtn.addEventListener('click', function () {
        toggleSearch();
      });
    }
    if (dom.searchCloseBtn) {
      dom.searchCloseBtn.addEventListener('click', function () {
        closeSearch();
      });
    }
    if (dom.searchPrevBtn) {
      dom.searchPrevBtn.addEventListener('click', function () {
        searchPrev();
      });
    }
    if (dom.searchNextBtn) {
      dom.searchNextBtn.addEventListener('click', function () {
        searchNext();
      });
    }
    if (dom.searchInput) {
      dom.searchInput.addEventListener('input', function () {
        state.searchQuery = dom.searchInput.value;
        state.searchActiveIndex = 1;
        applySearchAndFilter();
      });
      dom.searchInput.addEventListener('keydown', function (e) {
        if (e.key === 'Enter') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (e.shiftKey) {
            searchPrev();
          } else {
            searchNext();
          }
        } else if (e.key === 'Escape') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          closeSearch();
        }
      });
    }
    if (dom.filterAllBtn) {
      dom.filterAllBtn.addEventListener('click', function () {
        setSearchCategory('all');
      });
    }
    if (dom.filterCodeBtn) {
      dom.filterCodeBtn.addEventListener('click', function () {
        setSearchCategory('code_exe');
      });
    }
    if (dom.filterToolBtn) {
      dom.filterToolBtn.addEventListener('click', function () {
        setSearchCategory('tool');
      });
    }
    if (dom.filterThoughtBtn) {
      dom.filterThoughtBtn.addEventListener('click', function () {
        setSearchCategory('thought');
      });
    }
    if (dom.filterErrorBtn) {
      dom.filterErrorBtn.addEventListener('click', function () {
        setSearchCategory('error');
      });
    }
    if (dom.sidebarServerSelect) {
      dom.sidebarServerSelect.addEventListener('change', function () {
        if (dom.sidebarServerSelect.value) {
          var target = dom.sidebarServerSelect.value;
          if (dom.serverUrl) {
            dom.serverUrl.value = target;
          }
          setActiveServer(target);
        }
      });
    }

    if (dom.toggleTerminalBtn) {
      dom.toggleTerminalBtn.addEventListener('click', function () {
        toggleTerminal();
      });
    }
    if (dom.terminalCloseBtn) {
      dom.terminalCloseBtn.addEventListener('click', function () {
        setTerminalCollapsed(true);
      });
    }
    if (dom.terminalClearBtn) {
      dom.terminalClearBtn.addEventListener('click', function () {
        clearTerminalOutput();
      });
    }
    if (dom.terminalAddTabBtn) {
      dom.terminalAddTabBtn.addEventListener('click', addTerminalTab);
    }
    if (dom.terminalInput) {
      dom.terminalInput.addEventListener('keydown', handleTerminalKeydown);
    }
    if (typeof ResizeObserver !== 'undefined' && dom.terminalDrawer) {
      new ResizeObserver(emitTerminalResize).observe(dom.terminalDrawer);
    }
    if (dom.terminalForm) {
      dom.terminalForm.addEventListener('submit', function (e) {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        sendTerminalCommand();
      });
    }

    if (dom.hardRefreshBtn) {
      dom.hardRefreshBtn.addEventListener('click', function () {
        hardRefresh();
      });
    }

    document.addEventListener('click', function (e) {
      if (dom.exportMenu && !dom.exportMenu.hidden) {
        var inExport = false;
        var node = e.target;
        while (node) {
          if (node === dom.exportMenu || node === dom.exportToggleBtn) {
            inExport = true;
            break;
          }
          node = node.parentNode;
        }
        if (!inExport) {
          closeExportMenu();
        }
      }
    });

    if (dom.activeServerBtn) {
      dom.activeServerBtn.addEventListener('click', function (e) {
        if (typeof e.stopPropagation === 'function') e.stopPropagation();
        var isHidden = dom.serverProfileDropdown.hidden;
        dom.serverProfileDropdown.hidden = !isHidden;
        if (!dom.serverProfileDropdown.hidden) {
          renderServerDropdownList();
        }
      });
    }

    if (dom.scrollBottomBtn) {
      dom.scrollBottomBtn.addEventListener('click', scrollToBottom);
    }

    if (dom.messages) {
      dom.messages.addEventListener('scroll', handleMessagesScroll);
    }

    if (dom.closeShortcutsBtn) {
      dom.closeShortcutsBtn.addEventListener('click', closeShortcutsModal);
    }

    if (dom.commandPaletteInput) {
      dom.commandPaletteInput.addEventListener('input', function (e) {
        state.paletteActiveIndex = 0;
        renderCommandPaletteResults(e.target.value);
      });
      dom.commandPaletteInput.addEventListener('keydown', function (e) {
        var items = dom.commandPaletteResults ? dom.commandPaletteResults.querySelectorAll('.palette-item') : [];
        if (e.key === 'Escape') {
          closeCommandPalette();
        } else if (e.key === 'ArrowDown') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (items.length > 0) {
            state.paletteActiveIndex = (state.paletteActiveIndex + 1) % items.length;
            updatePaletteActiveItem();
          }
        } else if (e.key === 'ArrowUp') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (items.length > 0) {
            state.paletteActiveIndex = (state.paletteActiveIndex - 1 + items.length) % items.length;
            updatePaletteActiveItem();
          }
        } else if (e.key === 'Enter') {
          if (typeof e.preventDefault === 'function') e.preventDefault();
          if (items.length > 0 && items[state.paletteActiveIndex]) {
            items[state.paletteActiveIndex].click();
          }
        }
      });
    }

    document.addEventListener('keydown', function(e) {
      var activeModal = [
        dom.commandPaletteModal,
        dom.shortcutsModal,
        dom.filePreviewModal,
        dom.diffViewerModal,
        dom.diagnosticsModal,
        dom.agentApprovalModal
      ].find(function(m) { return m && !m.hidden; });

      if (activeModal && e.key === 'Tab') {
        var focusables = Array.prototype.slice.call(
          activeModal.querySelectorAll('button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])')
        );
        if (focusables.length > 0) {
          var firstEl = focusables[0];
          var lastEl = focusables[focusables.length - 1];
          if (e.shiftKey) {
            if (document.activeElement === firstEl || !activeModal.contains(document.activeElement)) {
              if (typeof e.preventDefault === 'function') e.preventDefault();
              lastEl.focus();
            }
          } else {
            if (document.activeElement === lastEl || !activeModal.contains(document.activeElement)) {
              if (typeof e.preventDefault === 'function') e.preventDefault();
              firstEl.focus();
            }
          }
        }
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        if (typeof e.preventDefault === 'function') e.preventDefault();
        openCommandPalette();
      } else if (e.key === '?' && !['INPUT', 'TEXTAREA'].includes(e.target.tagName)) {
        openShortcutsModal();
      } else if (e.key === 'Escape') {
        if (dom.commandPaletteModal && !dom.commandPaletteModal.hidden) {
          closeCommandPalette();
        } else if (dom.shortcutsModal && !dom.shortcutsModal.hidden) {
          closeShortcutsModal();
        } else if (dom.filePreviewModal && !dom.filePreviewModal.hidden) {
          closeFilePreview();
        } else if (dom.diffViewerModal && !dom.diffViewerModal.hidden) {
          closeDiffViewer();
        } else if (dom.diagnosticsModal && !dom.diagnosticsModal.hidden) {
          dom.diagnosticsModal.hidden = true;
        } else if (dom.agentApprovalModal && !dom.agentApprovalModal.hidden) {
          if (state.pendingApprovalResolver) state.pendingApprovalResolver(false, false);
        } else if (dom.serverProfileDropdown && !dom.serverProfileDropdown.hidden) {
          dom.serverProfileDropdown.hidden = true;
        } else if (dom.agentActivityDrawer && !dom.agentActivityDrawer.hidden) {
          dom.agentActivityDrawer.hidden = true;
        } else if (dom.exportMenu && !dom.exportMenu.hidden) {
          closeExportMenu();
        } else if (state.searchOpen) {
          closeSearch();
        }
      }
    });

    updateLatencySparkline(12);

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
      queueAndReport(text, messageId);
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
      if (err && (err.message === 'Session expired' || state.expired)) {
        return;
      }
      var record = state.logs.get(messageId);
      var row = state.rows.get(messageId);
      if (record) {
        record.queued = true;
        record.failed = false;
        if (row) {
          row.className = 'msg msg-user msg-queued';
        }
        paintRow(messageId);
      }
      queueAndReport(record || text, messageId);
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
    dom.searchToggleBtn = byId('search-toggle-btn');
    dom.searchDrawer = byId('search-drawer');
    dom.searchInput = byId('search-input');
    dom.searchCount = byId('search-count');
    dom.searchPrevBtn = byId('search-prev-btn');
    dom.searchNextBtn = byId('search-next-btn');
    dom.searchCloseBtn = byId('search-close-btn');
    dom.searchFilters = byId('search-filters');
    dom.filterAllBtn = byId('filter-all-btn');
    dom.filterCodeBtn = byId('filter-code-btn');
    dom.filterToolBtn = byId('filter-tool-btn');
    dom.filterThoughtBtn = byId('filter-thought-btn');
    dom.filterErrorBtn = byId('filter-error-btn');
    dom.exportToggleBtn = byId('export-toggle-btn');
    dom.exportMenu = byId('export-menu');
    dom.exportMdBtn = byId('export-md-btn');
    dom.exportJsonBtn = byId('export-json-btn');
    dom.copyTranscriptBtn = byId('copy-transcript-btn');

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
          var t = await store.get('app_theme');
          if (t) applyTheme(t);
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

    if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
      window.addEventListener('aqvn:reconnect', function () {
        disconnectSocket();
        stopPolling();
        connectSocket();
        flushOutbox();
      });
    }

    setConnection('offline');
    applyProgress(false, '');
    closeSearch();
    showLogin();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();