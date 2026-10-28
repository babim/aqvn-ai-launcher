import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

function createClassList() {
  const set = new Set();
  return {
    add: (name) => set.add(name),
    remove: (name) => set.delete(name),
    contains: (name) => set.has(name),
    toggle: (name, force) => {
      const on = force === undefined ? !set.has(name) : !!force;
      if (on) set.add(name);
      else set.delete(name);
      return on;
    },
    values: () => Array.from(set)
  };
}

function createElementMock(id) {
  const listeners = {};
  const attributes = {};
  const element = {
    id,
    value: '',
    textContent: '',
    innerHTML: '',
    hidden: false,
    disabled: false,
    dataset: {},
    style: {},
    attributes,
    classList: createClassList(),
    children: [],
    options: [],
    childElementCount: 0,
    get firstChild() {
      return element.children[0] || null;
    },
    get lastChild() {
      return element.children[element.children.length - 1] || null;
    },
    append: (...nodes) => {
      nodes.forEach((node) => element.appendChild(node));
    },
    focus: () => {},
    appendChild: (child) => {
      element.children.push(child);
      element.childElementCount = element.children.length;
      return child;
    },
    insertBefore: () => {},
    addEventListener: (type, fn) => {
      (listeners[type] = listeners[type] || []).push(fn);
    },
    removeEventListener: (type, fn) => {
      const list = listeners[type] || [];
      const idx = list.indexOf(fn);
      if (idx !== -1) list.splice(idx, 1);
    },
    dispatch: (type, event) => {
      (listeners[type] || []).slice().forEach((fn) => fn(event));
    },
    getAttribute: (name) => (name in attributes ? attributes[name] : null),
    setAttribute: (name, value) => {
      attributes[name] = String(value);
    },
    removeAttribute: (name) => {
      delete attributes[name];
    },
    hasAttribute: (name) => name in attributes,
    listeners
  };
  // Real DOM: assigning innerHTML replaces the subtree. Mirror that so
  // re-rendering a list cannot accumulate stale children in the mock.
  let innerHTML = '';
  Object.defineProperty(element, 'innerHTML', {
    get: () => innerHTML,
    set: (value) => {
      innerHTML = String(value);
      element.children.length = 0;
      element.childElementCount = 0;
    }
  });
  return element;
}

function createHarness(options = {}) {
  const filePath = new URL('../../ui/chat.js', import.meta.url);
  let code = fs.readFileSync(filePath, 'utf8');

  // Hook to extract functions before the closing IIFE. `eval` keeps the hook
  // tolerant: a name that does not exist yet resolves to undefined instead of
  // throwing, so RED runs report the missing behaviour, not a broken harness.
  const hook = `
__harness__.exports = (function () {
  var names = [
    'isLoginPage', 'apiCall', 'refreshCsrf', 'connectSocket', 'disconnectSocket',
    'requestState', 'startPolling', 'stopPolling', 'onServerRestart',
    'sessionExpired', 'state', 'dom', 'showChatError', 'init', 'initLayout',
    'toggleSidebar', 'toggleCanvas', 'handleShortcut', 'applySidebarWidth',
    'applyCanvasWidth', 'setSidebarCollapsed', 'setCanvasCollapsed',
    'renderContent', 'copyCode', 'applyProgress', 'notifyComplete',
    'setConnection', 'sendMessage', 'queueOutbox', 'readOutbox',
    'flushOutbox', 'applyZoom', 'readZoom', 'clampZoom', 'hardRefresh',
    'reloadCanvas', 'writeOutbox', 'OUTBOX_MAX_ENTRIES', 'OUTBOX_MAX_BYTES'
  ];
  var out = {};
  names.forEach(function (name) {
    try {
      out[name] = eval(name);
    } catch (err) {
      out[name] = undefined;
    }
  });
  return out;
})();
`;
  const lastIndex = code.lastIndexOf('})();');
  if (lastIndex !== -1) {
    code = code.slice(0, lastIndex) + hook + '\n})();';
  }

  const elements = {};
  function getEl(id) {
    if (!elements[id]) {
      elements[id] = createElementMock(id);
    }
    return elements[id];
  }

  const timeouts = [];
  const fakeSetTimeout = (fn, delay) => {
    const id = timeouts.length + 1;
    timeouts.push({ id, fn, delay });
    return id;
  };
  const fakeClearTimeout = (id) => {
    const idx = timeouts.findIndex((t) => t.id === id);
    if (idx !== -1) timeouts.splice(idx, 1);
  };

  const listeners = {};
  const mockIo = {
    connected: true,
    connect: () => {},
    on: (evt, handler) => {
      listeners[evt] = handler;
    },
    emit: (evt, data, cb) => {},
    removeAllListeners: () => {},
    disconnect: () => {}
  };

  const docListeners = {};
  const documentMock = {
    title: '',
    readyState: 'loading',
    body: { classList: createClassList(), style: {} },
    getElementById: getEl,
    createElement: () => createElementMock('el'),
    createTextNode: () => ({}),
    addEventListener: (type, fn) => {
      (docListeners[type] = docListeners[type] || []).push(fn);
    },
    removeEventListener: (type, fn) => {
      const list = docListeners[type] || [];
      const idx = list.indexOf(fn);
      if (idx !== -1) list.splice(idx, 1);
    },
    fire: (type, event) => {
      (docListeners[type] || []).slice().forEach((fn) => fn(event));
    }
  };

  const store = {};
  const localStorageMock = {
    getItem: (key) => (key in store ? store[key] : null),
    setItem: (key, value) => {
      if (options.storageThrows) {
        throw new Error('QuotaExceededError: storage is unavailable');
      }
      store[key] = String(value);
    },
    removeItem: (key) => {
      if (options.storageThrows) {
        throw new Error('SecurityError: storage is unavailable');
      }
      delete store[key];
    }
  };

  const context = {
    __harness__: {},
    console,
    Intl,
    crypto,
    fetch: null,
    Headers,
    Response,
    URLSearchParams,
    document: documentMock,
    window: {
      location: { origin: 'http://localhost:3000', search: '' },
      localStorage: localStorageMock,
      setTimeout: fakeSetTimeout,
      clearTimeout: fakeClearTimeout,
      __AQVN_PROXY__: null,
      bridge: null
    },
    io: (url, opts) => {
      mockIo.authOpts = opts;
      return mockIo;
    }
  };

  vm.createContext(context);

  // Vendor libraries define window.marked / window.hljs in the webview. Load
  // them into the same sandbox so renderContent can be exercised for real.
  ['marked.min.js', 'highlight.min.js'].forEach((name) => {
    const vendorPath = new URL(`../../ui/vendor/${name}`, import.meta.url);
    vm.runInContext(fs.readFileSync(vendorPath, 'utf8'), context);
  });

  vm.runInContext(code, context);

  // Initialize dom references needed by chat.js
  const dom = context.__harness__.exports.dom;
  [
    'connection', 'chatError', 'sessionNotice', 'loginView', 'chatView',
    'password', 'username', 'input', 'status', 'stop'
  ].forEach((id) => {
    dom[id] = getEl(id);
  });

  return {
    exports: context.__harness__.exports,
    context,
    timeouts,
    mockIo,
    listeners,
    getEl,
    store,
    document: documentMock
  };
}

describe('ui/chat.js false session expiration & NO_HANDLERS hardening', () => {
  test('isLoginPage returns false on 502/500/504 HTML error pages and true on 200/401 HTML', () => {
    const { exports } = createHarness();
    const { isLoginPage } = exports;

    // Upstream 502 Bad Gateway with HTML error body must NOT be considered a login page
    const res502 = {
      status: 502,
      headers: new Headers({ 'content-type': 'text/html; charset=utf-8' })
    };
    assert.equal(isLoginPage(res502), false, '502 with text/html must return false');

    // Upstream 500 Internal Server Error
    const res500 = {
      status: 500,
      headers: new Headers({ 'content-type': 'text/html' })
    };
    assert.equal(isLoginPage(res500), false, '500 with text/html must return false');

    // Upstream 404 Not Found
    const res404 = {
      status: 404,
      headers: new Headers({ 'content-type': 'text/html' })
    };
    assert.equal(isLoginPage(res404), false, '404 with text/html must return false');

    // 200 OK redirected to HTML login page
    const res200Html = {
      status: 200,
      headers: new Headers({ 'content-type': 'text/html' })
    };
    assert.equal(isLoginPage(res200Html), true, '200 with text/html must return true');

    // 401 Unauthorized with HTML login page
    const res401Html = {
      status: 401,
      headers: new Headers({ 'content-type': 'text/html' })
    };
    assert.equal(isLoginPage(res401Html), true, '401 with text/html must return true');

    // 200 OK JSON
    const res200Json = {
      status: 200,
      headers: new Headers({ 'content-type': 'application/json' })
    };
    assert.equal(isLoginPage(res200Json), false, '200 with application/json must return false');
  });

  test('apiCall throws on JSON parse error without calling sessionExpired', async () => {
    const harness = createHarness();
    const { apiCall, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.csrf = 'valid-token';
    state.expired = false;

    // Mock fetch returning invalid JSON
    harness.context.fetch = async () => ({
      status: 200,
      ok: true,
      headers: new Headers({ 'content-type': 'application/json' }),
      json: async () => {
        throw new SyntaxError('Unexpected token < in JSON at position 0');
      }
    });

    await assert.rejects(
      async () => {
        await apiCall('test_endpoint', {});
      },
      (err) => {
        assert.notEqual(err.message, 'Session expired');
        return true;
      }
    );

    assert.equal(state.expired, false, 'sessionExpired must NOT be called on JSON parse failure');
  });

  test('state_request ack handles NO_HANDLERS by starting polling and bypassing sessionExpired', () => {
    const harness = createHarness();
    const { requestState, state } = harness.exports;
    state.expired = false;
    state.polling = false;

    let emittedEvent = null;
    let ackCallback = null;
    state.socket = {
      connected: true,
      emit: (evt, data, cb) => {
        emittedEvent = evt;
        ackCallback = cb;
      },
      removeAllListeners: () => {},
      disconnect: () => {}
    };

    requestState();
    assert.equal(emittedEvent, 'state_request');
    assert.ok(ackCallback, 'Ack callback should have been registered');

    // 1. Nested ack.results[0] with code: 'NO_HANDLERS' mentioning 'session'
    ackCallback({
      results: [
        {
          ok: false,
          error: {
            code: 'NO_HANDLERS',
            error: 'No session state handler registered'
          }
        }
      ]
    });

    assert.equal(state.expired, false, 'NO_HANDLERS must NOT expire session');
    assert.equal(state.polling, true, 'NO_HANDLERS must trigger startPolling');

    // 2. Unnested ack with text 'no handlers'
    state.expired = false;
    state.polling = false;
    ackCallback({
      ok: false,
      error: 'no handlers available for state_request'
    });

    assert.equal(state.expired, false, 'no handlers in text must NOT expire session');
    assert.equal(state.polling, true, 'no handlers in text must trigger startPolling');

    // 3. Genuine auth error DOES expire session
    state.expired = false;
    state.polling = false;
    ackCallback({
      results: [
        {
          ok: false,
          error: {
            code: 'SESSION_EXPIRED',
            error: 'Invalid session token'
          }
        }
      ]
    });

    assert.equal(state.expired, true, 'Genuine auth error must expire session');
  });

  test('connectSocket auth callback does not expire session on transient 502/network failure', async () => {
    const harness = createHarness();
    const { connectSocket, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.everConnected = true;
    state.expired = false;

    // Mock fetch for refreshCsrf returning 502
    harness.context.fetch = async () => ({
      status: 502,
      ok: false
    });

    connectSocket();
    const authFn = harness.mockIo.authOpts.auth;
    assert.ok(typeof authFn === 'function');

    await new Promise((resolve) => {
      authFn((authData) => {
        resolve(authData);
      });
    });

    assert.equal(state.expired, false, '502 error during CSRF refresh must not expire session');

    // But 401 unauthorized DOES expire session
    harness.context.fetch = async () => ({
      status: 401,
      ok: false
    });

    await new Promise((resolve) => {
      authFn((authData) => {
        resolve(authData);
      });
    });

    assert.equal(state.expired, true, '401 Unauthorized during CSRF refresh must expire session');
  });

  test('startPolling does not permanently stop polling on transient error', async () => {
    const harness = createHarness();
    const { startPolling, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.csrf = 'valid-token';
    state.expired = false;
    state.polling = false;

    // Mock fetch returning transient error (e.g. 502)
    harness.context.fetch = async () => ({
      status: 502,
      ok: false,
      text: async () => 'Bad Gateway'
    });

    startPolling();
    assert.equal(state.polling, true, 'Polling should be active');

    // Execute the initial tick
    const tick = harness.timeouts[0];
    assert.ok(tick, 'Initial tick timeout must be scheduled');
    tick.fn();

    // Wait a tick for promise rejection
    await new Promise((resolve) => setTimeout(resolve, 20));

    assert.equal(
      state.polling,
      true,
      'Transient 502 in poll should not permanently stop polling'
    );
  });

  test('startPolling sends context: null when state.context is empty or falsy', async () => {
    const harness = createHarness();
    const { startPolling, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.csrf = 'valid-token';
    state.expired = false;
    state.polling = false;
    state.context = '';

    let pollPayload = null;
    harness.context.fetch = async (url, options) => {
      if (url.includes('/api/poll')) {
        pollPayload = JSON.parse(options.body);
      }
      return {
        status: 200,
        ok: true,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => ({})
      };
    };

    startPolling();
    const tick = harness.timeouts[0];
    assert.ok(tick);
    tick.fn();

    await new Promise((resolve) => setTimeout(resolve, 20));

    assert.ok(pollPayload, 'apiCall poll should have been made');
    assert.equal(
      pollPayload.context,
      null,
      'poll payload context must be null when state.context is empty string'
    );
  });

  test('onServerRestart does not immediately expire session on transient 502', async () => {
    const harness = createHarness();
    const { onServerRestart, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.expired = false;

    // Mock fetch for refreshCsrf returning 502
    harness.context.fetch = async () => ({
      status: 502,
      ok: false
    });

    onServerRestart();

    // Wait for promise rejection
    await new Promise((resolve) => setTimeout(resolve, 20));

    assert.equal(
      state.expired,
      false,
      'Transient 502 on restart should not immediately expire session'
    );
    // Should have scheduled a reconnect retry timeout
    assert.ok(
      harness.timeouts.length > 0,
      'Retry timeout should be scheduled on transient restart error'
    );
  });
});

/* ------------------------------------------------------------------ */
/* VAL-LAYOUT-001..004: 3-column shell, splitters, panel toggles      */
/* ------------------------------------------------------------------ */

function readIndexHtml() {
  return fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
}

/** Extract the element subtree that starts at `id="<id>"`, honouring nesting. */
function sectionOf(html, id) {
  const marker = `id="${id}"`;
  const at = html.indexOf(marker);
  assert.notEqual(at, -1, `ui/index.html must contain an element with id="${id}"`);
  const tagStart = html.lastIndexOf('<', at);
  const tag = html.slice(tagStart, html.indexOf('>', tagStart));
  const name = (tag.match(/^<([a-z0-9]+)/i) || [])[1];
  assert.ok(name, `id="${id}" must live in a normal element`);

  // Walk forward counting open/close tags of the same name so nested
  // same-name elements (e.g. div inside div) do not truncate the subtree.
  const open = new RegExp(`<${name}(?=[\\s>])`, 'gi');
  const close = new RegExp(`</${name}\\s*>`, 'gi');
  let depth = 0;
  let cursor = tagStart;
  while (cursor < html.length) {
    open.lastIndex = cursor;
    close.lastIndex = cursor;
    const nextOpen = open.exec(html);
    const nextClose = close.exec(html);
    if (!nextClose) {
      break;
    }
    if (nextOpen && nextOpen.index < nextClose.index) {
      depth += 1;
      cursor = nextOpen.index + nextOpen[0].length;
      continue;
    }
    depth -= 1;
    cursor = nextClose.index + nextClose[0].length;
    if (depth === 0) {
      return html.slice(tagStart, cursor);
    }
  }
  assert.fail(`element with id="${id}" must be closed`);
}

describe('ui/index.html 3-column layout, splitters and panel toggles', () => {
  test('VAL-LAYOUT-001: index.html renders sidebar, chat pane, canvas pane and both splitters', () => {
    const html = readIndexHtml();

    ['sidebar', 'chat-pane', 'canvas-pane', 'splitter-left', 'splitter-right'].forEach((id) => {
      assert.ok(html.includes(`id="${id}"`), `ui/index.html must declare #${id}`);
    });

    const sidebar = sectionOf(html, 'sidebar');
    assert.ok(sidebar.includes('id="contexts"'), '#sidebar must hold the session context list');
    assert.ok(sidebar.includes('id="new-chat-btn"'), '#sidebar must hold the New Chat button');
    assert.ok(sidebar.includes('id="sidebar-server-select"'), '#sidebar must hold the Server Switcher');

    const chatPane = sectionOf(html, 'chat-pane');
    assert.ok(chatPane.includes('id="messages"'), '#chat-pane must hold the message stream');
    assert.ok(chatPane.includes('id="composer"'), '#chat-pane must hold the composer');

    const canvas = sectionOf(html, 'canvas-pane');
    assert.ok(canvas.includes('Intel Suite'), "#canvas-pane must show the 'Intel Suite' tab header");
    assert.ok(canvas.includes('id="close-canvas-btn"'), '#canvas-pane must hold the close button');
    assert.ok(canvas.includes('id="canvas-frame"'), '#canvas-pane must hold the canvas iframe');
  });

  test('VAL-LAYOUT-002: Alt+S and #toggle-sidebar-btn collapse #sidebar and persist the state', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    assert.equal(dom.sidebar.getAttribute('hidden'), null, 'sidebar starts expanded');

    harness.document.fire('keydown', { altKey: true, key: 's', preventDefault: () => {} });
    assert.equal(dom.sidebar.getAttribute('hidden'), '', 'Alt+S must hide #sidebar');
    assert.ok(dom.sidebar.classList.contains('collapsed'), 'Alt+S must mark #sidebar collapsed');
    assert.equal(
      harness.store['aqvn_sidebar_collapsed'],
      'true',
      'collapsed state must persist in localStorage[aqvn_sidebar_collapsed]'
    );

    harness.document.fire('keydown', { altKey: true, key: 's', preventDefault: () => {} });
    assert.equal(dom.sidebar.getAttribute('hidden'), null, 'Alt+S must show #sidebar again');
    assert.equal(
      harness.store['aqvn_sidebar_collapsed'],
      'false',
      'expanded state must persist in localStorage[aqvn_sidebar_collapsed]'
    );

    dom.toggleSidebarBtn.dispatch('click', {});
    assert.equal(dom.sidebar.getAttribute('hidden'), '', '#toggle-sidebar-btn must hide #sidebar');
    assert.equal(harness.store['aqvn_sidebar_collapsed'], 'true');
  });

  test('VAL-LAYOUT-003: Alt+C and #toggle-canvas-btn collapse #canvas-pane and load the Intel Suite iframe', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    harness.document.fire('keydown', { altKey: true, key: 'c', preventDefault: () => {} });
    assert.equal(dom.canvasPane.getAttribute('hidden'), '', 'Alt+C must hide #canvas-pane');
    assert.ok(dom.canvasPane.classList.contains('collapsed'), 'Alt+C must mark #canvas-pane collapsed');
    assert.equal(
      harness.store['aqvn_canvas_collapsed'],
      'true',
      'collapsed state must persist in localStorage[aqvn_canvas_collapsed]'
    );

    harness.document.fire('keydown', { altKey: true, key: 'c', preventDefault: () => {} });
    assert.equal(dom.canvasPane.getAttribute('hidden'), null, 'Alt+C must show #canvas-pane again');
    assert.equal(
      dom.canvasFrame.getAttribute('src'),
      '/plugins/aqvn_intel_suite/webui/canvas.html',
      'opening the canvas must load the Intel Suite page in #canvas-frame'
    );
    assert.equal(harness.store['aqvn_canvas_collapsed'], 'false');

    dom.toggleCanvasBtn.dispatch('click', {});
    assert.equal(dom.canvasPane.getAttribute('hidden'), '', '#toggle-canvas-btn must hide #canvas-pane');

    dom.closeCanvasBtn.dispatch('click', {});
    assert.equal(dom.canvasPane.getAttribute('hidden'), '', '#close-canvas-btn must close #canvas-pane');
  });

  test('VAL-LAYOUT-004: dragging the splitters resizes #sidebar and #canvas-pane within bounds', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    // Left splitter: dragging left widens the sidebar, clamped to [160px, 400px].
    dom.splitterLeft.dispatch('mousedown', { button: 0, clientX: 220, preventDefault: () => {} });
    harness.document.fire('mousemove', { clientX: 20 });
    assert.equal(dom.sidebar.style.width, '160px', 'sidebar must clamp to the 160px minimum');

    harness.document.fire('mousemove', { clientX: 1200 });
    assert.equal(dom.sidebar.style.width, '400px', 'sidebar must clamp to the 400px maximum');

    harness.document.fire('mouseup', {});

    // Right splitter: dragging left widens the canvas, clamped to [280px, 800px].
    dom.splitterRight.dispatch('mousedown', { button: 0, clientX: 450, preventDefault: () => {} });
    harness.document.fire('mousemove', { clientX: 900 });
    assert.equal(dom.canvasPane.style.width, '280px', 'canvas must clamp to the 280px minimum');

    harness.document.fire('mousemove', { clientX: -900 });
    assert.equal(dom.canvasPane.style.width, '800px', 'canvas must clamp to the 800px maximum');
  });

  test('sidebar server switcher lists the active server profiles', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    assert.ok(dom.sidebarServerSelect, 'the sidebar must expose a server switcher');
    assert.equal(dom.sidebarServerSelect.children.length, 1);
    assert.equal(dom.sidebarServerSelect.children[0].value, 'https://a0.matmagoc.com');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-MD-002..004: Markdown, highlight, copy and terminal styling    */
/* ------------------------------------------------------------------ */

function readChatCss() {
  return fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
}

function readChatJs() {
  return fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
}

describe('ui/chat.js Markdown rendering and terminal code styling', () => {
  test('VAL-MD-002: renderContent parses Markdown into structured HTML', () => {
    const harness = createHarness();
    const { renderContent } = harness.exports;
    assert.equal(typeof renderContent, 'function', 'renderContent must exist');

    const node = harness.getEl('md-body');
    renderContent(
      node,
      '# Title\n\n**bold** and *italic*\n\n- one\n- two\n\n> quoted\n\n[link](https://example.com)\n'
    );

    const html = node.innerHTML;
    assert.match(html, /<h1>Title<\/h1>/, 'headers must render as <h1>');
    assert.match(html, /<strong>bold<\/strong>/, 'bold must render as <strong>');
    assert.match(html, /<em>italic<\/em>/, 'italic must render as <em>');
    assert.match(html, /<ul>[\s\S]*<li>one<\/li>[\s\S]*<li>two<\/li>/, 'lists must render as <ul>/<li>');
    assert.match(html, /<blockquote>/, 'blockquotes must render as <blockquote>');
    assert.match(
      html,
      /<a href="https:\/\/example\.com" rel="noopener noreferrer">link<\/a>/,
      'links must render as anchors with rel=noopener'
    );
    assert.doesNotMatch(html, /\*\*bold\*\*/, 'Markdown markers must not leak as plain text');
  });

  test('VAL-MD-002: renderContent escapes raw HTML and unsafe URLs', () => {
    const harness = createHarness();
    const { renderContent } = harness.exports;
    const node = harness.getEl('md-safe');

    renderContent(node, '<img src=x onerror="alert(1)"> [bad](javascript:alert(1))');
    const html = node.innerHTML;

    assert.doesNotMatch(html, /<img/i, 'raw HTML must never reach the DOM as markup');
    assert.match(html, /&lt;img/, 'raw HTML must be escaped to text');
    assert.doesNotMatch(html, /href="javascript:/i, 'javascript: links must be dropped');
    assert.match(html, /bad/, 'the link label survives even when the URL is rejected');
  });

  test('VAL-MD-002: a two-line plain message keeps its line break', () => {
    const harness = createHarness();
    const { renderContent } = harness.exports;
    const node = harness.getEl('md-breaks');

    renderContent(node, 'first line\nsecond line');
    const html = node.innerHTML;

    assert.match(html, /first line/, 'the first line must be rendered');
    assert.match(html, /<br\s*\/?>/, 'a single newline must become a <br>');
    assert.match(html, /second line/, 'the second line must be rendered');
    assert.ok(
      html.indexOf('first line') < html.indexOf('<br'),
      'the break must sit between the two lines'
    );
  });

  test('VAL-MD-003: code blocks are highlighted and decorated with language badge and Copy button', () => {
    const harness = createHarness();
    const { renderContent } = harness.exports;
    const node = harness.getEl('md-code');

    renderContent(node, '```js\nconst answer = 42;\n```\n');
    const html = node.innerHTML;

    assert.match(html, /class="code-block"/, 'code blocks must be wrapped in .code-block');
    assert.match(html, /class="code-lang">js<\/span>/, 'language badge must show the fence language');
    assert.match(html, /class="code-copy"[^>]*>Copy<\/button>/, 'a Copy button must be present');
    assert.match(html, /<pre><code class="hljs language-js">/, 'code must carry hljs language classes');
    assert.match(html, /<span class="hljs-keyword">const<\/span>/, 'highlight.js must tokenize the code');
    assert.match(html, /<span class="hljs-number">42<\/span>/, 'highlight.js must color literals');
  });

  test('VAL-MD-003: Copy button writes the code to navigator.clipboard and shows feedback', async () => {
    const harness = createHarness();
    const { copyCode } = harness.exports;
    assert.equal(typeof copyCode, 'function', 'copyCode must exist');

    let copied = null;
    harness.context.navigator = {
      clipboard: {
        writeText: (value) => {
          copied = value;
          return Promise.resolve();
        }
      }
    };

    const button = {
      textContent: 'Copy',
      classList: createClassList(),
      parentNode: null
    };
    const block = {
      classList: createClassList(),
      querySelector: () => ({ textContent: 'const answer = 42;' }),
      parentNode: null
    };
    block.classList.add('code-block');
    button.parentNode = block;

    copyCode(button);
    // Cross-realm promises settle over a couple of macrotasks.
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));

    assert.equal(copied, 'const answer = 42;', 'clipboard.writeText must receive the raw code');
    assert.equal(button.textContent, 'Copied!', 'button must show copied feedback');
    assert.ok(button.classList.contains('copied'), 'button must expose the copied state');

    const reset = harness.timeouts.find((t) => t.delay === 1200);
    assert.ok(reset, 'feedback must reset on a timer');
    reset.fn();
    assert.equal(button.textContent, 'Copy', 'button label must return to Copy');
  });

  test('VAL-MD-004: code_exe and tool messages use the #0d1117 terminal theme and $ EXECUTE branding', () => {
    const css = readChatCss();

    assert.match(
      css,
      /\.msg-code_exe[^{]*\{[^}]*#0d1117/i,
      'msg-code_exe must paint the #0d1117 terminal background'
    );
    assert.match(
      css,
      /\.msg-tool[^{]*\{[^}]*#0d1117/i,
      'msg-tool must paint the #0d1117 terminal background'
    );
    assert.match(
      css,
      /\.msg-code_exe\s+\.msg-head[^{]*\{[^}]*var\(--accent\)/i,
      'code_exe header branding must be cyan'
    );
    assert.match(
      css,
      /\.msg-tool\s+\.msg-head[^{]*\{[^}]*var\(--accent\)/i,
      'tool header branding must be cyan'
    );
    assert.ok(readChatJs().includes("'$ EXECUTE'"), 'code_exe/tool rows must show $ EXECUTE');
  });

  test("VAL-MD-004: thought content renders in <details class='thought-step'>", () => {
    const harness = createHarness();
    const { renderContent } = harness.exports;
    const node = harness.getEl('md-thought');

    renderContent(node, 'Checking **facts** first', 'thought');
    const html = node.innerHTML;

    assert.match(html, /^<details class="thought-step">/, 'thought content must be collapsible');
    assert.match(html, /<summary>Thought<\/summary>/, 'details must expose a summary');
    assert.match(html, /<strong>facts<\/strong>/, 'thought body must still parse Markdown');
    assert.match(html, /<\/details>$/, 'details must be closed');

    renderContent(node, 'plain answer', 'agent');
    assert.doesNotMatch(node.innerHTML, /thought-step/, 'normal messages must not be collapsed');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-FOCUS-001: focus mode toggle, layout and composer focus        */
/* ------------------------------------------------------------------ */

describe('ui/chat.js Focus Mode and hotkeys', () => {
  test('VAL-FOCUS-001: F11 and Ctrl+Shift+F toggle .focus-mode and focus the composer', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;
    const body = harness.document.body;
    let focused = 0;
    dom.input.focus = () => {
      focused += 1;
    };
    const key = (init) => Object.assign({ preventDefault: () => {} }, init);

    assert.equal(body.classList.contains('focus-mode'), false, 'focus mode starts off');

    harness.document.fire('keydown', key({ key: 'F11' }));
    assert.ok(body.classList.contains('focus-mode'), 'F11 must enable focus mode');
    assert.equal(focused, 1, 'entering focus mode must focus #message-input');

    harness.document.fire('keydown', key({ key: 'F11' }));
    assert.equal(body.classList.contains('focus-mode'), false, 'F11 must toggle focus mode off');

    harness.document.fire('keydown', key({ key: 'f', ctrlKey: true, shiftKey: true }));
    assert.ok(body.classList.contains('focus-mode'), 'Ctrl+Shift+F must enable focus mode');
    assert.equal(focused, 2, 'Ctrl+Shift+F must focus #message-input');

    harness.document.fire('keydown', key({ key: 'F', ctrlKey: true, shiftKey: true }));
    assert.equal(body.classList.contains('focus-mode'), false, 'Ctrl+Shift+F must toggle focus mode off');
  });

  test('VAL-FOCUS-001: .focus-mode hides side panels and centers #chat-pane at 840px', () => {
    const css = readChatCss();

    assert.match(
      css,
      /\.focus-mode\s+#sidebar/i,
      '.focus-mode must hide #sidebar'
    );
    assert.match(
      css,
      /\.focus-mode\s+#canvas-pane/i,
      '.focus-mode must hide #canvas-pane'
    );
    assert.match(
      css,
      /\.focus-mode\s+\.splitter/i,
      '.focus-mode must hide the splitters'
    );
    assert.match(
      css,
      /\.focus-mode\s+#chat-pane[^{]*\{[^}]*max-width:\s*840px/i,
      '.focus-mode must center #chat-pane with max-width: 840px'
    );
  });
});

/* ------------------------------------------------------------------ */
/* VAL-CONN-001..003: notification, LED states, offline outbox        */
/* ------------------------------------------------------------------ */

function tauriRecorder(harness) {
  const calls = [];
  harness.context.window.__TAURI_INTERNALS__ = {
    invoke: (command, args) => {
      calls.push({ command, args });
      return Promise.resolve();
    }
  };
  return calls;
}

function flushPromises() {
  return new Promise((resolve) => setImmediate(resolve));
}

describe('ui/chat.js notifications, connection LED and offline outbox', () => {
  test('VAL-CONN-001: notify_task_complete fires only when the task ends while hidden', () => {
    const harness = createHarness();
    const calls = tauriRecorder(harness);
    const { applyProgress } = harness.exports;
    const notes = () => calls.filter((c) => c.command === 'notify_task_complete');

    harness.document.hidden = false;
    applyProgress(true, 'working');
    applyProgress(false, 'done');
    assert.equal(notes().length, 0, 'visible window must not raise a notification');

    harness.document.hidden = true;
    applyProgress(true, 'working again');
    applyProgress(false, 'all done');
    assert.equal(notes().length, 1, 'hidden window must raise exactly one notification');
    assert.equal(notes()[0].args.title, 'Task complete');
    assert.match(notes()[0].args.body, /all done/);
  });

  test('VAL-CONN-002: #connection-pill renders 🟢 Online, 🟡 Polling and 🔴 Offline', () => {
    const harness = createHarness();
    harness.exports.init();
    const { setConnection } = harness.exports;
    const pill = harness.getEl('connection-pill');

    setConnection('online');
    assert.equal(pill.textContent, '🟢 Online');
    assert.equal(pill.dataset.state, 'online');

    setConnection('polling');
    assert.equal(pill.textContent, '🟡 Polling');
    assert.equal(pill.dataset.state, 'polling');

    setConnection('offline');
    assert.equal(pill.textContent, '🔴 Offline');
    assert.equal(pill.dataset.state, 'offline');
  });

  test('VAL-CONN-002: socket connect and polling fallback drive the LED states', () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket, startPolling, stopPolling } = harness.exports;
    const pill = harness.getEl('connection-pill');

    connectSocket();
    harness.listeners.connect();
    assert.equal(pill.dataset.state, 'online', 'socket connect must show online');
    assert.equal(pill.textContent, '🟢 Online');

    stopPolling();
    startPolling();
    assert.equal(pill.dataset.state, 'polling', 'poll fallback must show polling');
    assert.equal(pill.textContent, '🟡 Polling');
    stopPolling();
  });

  test('VAL-CONN-002: index.html exposes #connection-pill and CSS styles all three states', () => {
    const html = readIndexHtml();
    assert.match(html, /id="connection-pill"/, 'index.html must expose #connection-pill');

    const css = readChatCss();
    assert.match(css, /\.conn\[data-state="online"\]/, 'CSS must style the online state');
    assert.match(css, /\.conn\[data-state="polling"\]/, 'CSS must style the polling state');
    assert.match(css, /\.conn\[data-state="offline"\]/, 'CSS must style the offline state');
  });

  test('VAL-CONN-003: sending while offline queues into localStorage aqvn_outbox', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, sendMessage, dom } = harness.exports;
    state.online = false;
    state.polling = false;

    dom.input.value = 'queued hello';
    await sendMessage({ preventDefault: () => {} });

    const raw = harness.store['aqvn_outbox'];
    assert.ok(raw, 'offline message must be persisted in aqvn_outbox');
    const queued = JSON.parse(raw);
    assert.equal(queued.length, 1);
    assert.equal(queued[0].text, 'queued hello');

    const id = queued[0].message_id;
    assert.ok(id, 'queued message must keep its message_id');
    const record = state.logs.get(id);
    assert.ok(record, 'queued message must render optimistically');
    assert.equal(record.queued, true);

    const row = state.rows.get(id);
    assert.match(row.className, /msg-queued/, 'row must carry the offline indicator class');
    assert.match(row.firstChild.textContent, /queued/i, 'header must label the offline queue');
  });

  test('VAL-CONN-003: socket connect flushes the outbox and clears storage', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket } = harness.exports;
    harness.store['aqvn_outbox'] = JSON.stringify([
      { text: 'later', context: 'ctx-1', message_id: 'm-1', queued_at: 1 }
    ]);

    const sent = [];
    harness.context.fetch = async (url, options) => {
      if (String(url).includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ token: 't' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      sent.push(JSON.parse(options.body));
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    connectSocket();
    harness.listeners.connect();
    await flushPromises();
    await flushPromises();

    assert.equal(sent.length, 1, 'queued message must be sent on connect');
    assert.equal(sent[0].text, 'later');
    assert.equal(sent[0].context, 'ctx-1');
    assert.equal(sent[0].message_id, 'm-1');
    assert.equal(harness.store['aqvn_outbox'], undefined, 'outbox must clear after flush');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-ZOOM-001..002: UI zoom and hard refresh                        */
/* ------------------------------------------------------------------ */

function stubCsrfFetch(harness) {
  const calls = { csrf: 0 };
  harness.context.fetch = async (url) => {
    if (String(url).includes('/proxy/csrf')) {
      calls.csrf += 1;
      return new Response(JSON.stringify({ token: 'csrf-' + calls.csrf }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    }
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { 'content-type': 'application/json' }
    });
  };
  return calls;
}

describe('ui/chat.js UI zoom and hard refresh', () => {
  test('VAL-ZOOM-001: Ctrl +/-/0 adjust body zoom within [0.7, 2.0] and persist it', () => {
    const harness = createHarness();
    harness.exports.init();
    const body = harness.document.body;
    const key = (init) => Object.assign({ preventDefault: () => {} }, init);

    harness.document.fire('keydown', key({ key: '=', ctrlKey: true }));
    assert.equal(body.style.zoom, '1.1', 'Ctrl + must zoom in by one step');
    harness.document.fire('keydown', key({ key: '+', ctrlKey: true }));
    assert.equal(body.style.zoom, '1.2', 'Ctrl + must accumulate zoom steps');
    harness.document.fire('keydown', key({ key: '-', ctrlKey: true }));
    assert.equal(body.style.zoom, '1.1', 'Ctrl - must zoom out by one step');
    harness.document.fire('keydown', key({ key: '0', ctrlKey: true }));
    assert.equal(body.style.zoom, '1', 'Ctrl 0 must reset zoom to 1.0');
    assert.equal(harness.store['aqvn_zoom'], '1', 'zoom must persist in localStorage aqvn_zoom');

    const { applyZoom, clampZoom } = harness.exports;
    assert.equal(typeof applyZoom, 'function', 'applyZoom must exist');
    assert.equal(clampZoom(9), 2, 'zoom clamps at 2.0');
    assert.equal(clampZoom(0.1), 0.7, 'zoom clamps at 0.7');
    assert.equal(applyZoom(9), 2, 'applyZoom returns the clamped value');
    assert.equal(body.style.zoom, '2', 'clamped zoom is applied to document.body');
    assert.equal(harness.store['aqvn_zoom'], '2', 'clamped zoom is persisted');
  });

  test('VAL-ZOOM-001: stored aqvn_zoom is applied on init', () => {
    const harness = createHarness();
    harness.store['aqvn_zoom'] = '1.5';
    harness.exports.init();
    assert.equal(harness.document.body.style.zoom, '1.5', 'init must apply the stored zoom');
  });

  test('VAL-ZOOM-002: #hard-refresh-btn resets RAM state, CSRF, socket and canvas iframe', async () => {
    const harness = createHarness();
    harness.exports.init();
    const csrf = stubCsrfFetch(harness);
    const { state } = harness.exports;

    state.logs.set('stale', { id: 'stale', content: 'old' });
    state.order.push('stale');
    state.rows.set('stale', {});

    let ioCalls = 0;
    harness.context.io = () => {
      ioCalls += 1;
      return harness.mockIo;
    };

    harness.getEl('hard-refresh-btn').dispatch('click', { preventDefault: () => {} });
    await flushPromises();
    await flushPromises();

    assert.equal(state.logs.size, 0, 'hard refresh must clear RAM logs');
    assert.equal(state.order.length, 0, 'hard refresh must clear the log order');
    assert.equal(state.rows.size, 0, 'hard refresh must clear rendered rows');
    assert.equal(csrf.csrf, 1, 'hard refresh must re-fetch CSRF exactly once');
    assert.equal(state.csrf, 'csrf-1', 'hard refresh must store the fresh CSRF token');
    assert.equal(ioCalls, 1, 'hard refresh must reconnect Socket.IO');
    assert.equal(
      harness.getEl('canvas-frame').getAttribute('src'),
      state.base + '/plugins/aqvn_intel_suite/webui/canvas.html',
      'hard refresh must reload the canvas iframe'
    );
  });

  test('VAL-ZOOM-002: Ctrl+Shift+R and F5 trigger the hard refresh', async () => {
    for (const init of [{ key: 'r', ctrlKey: true, shiftKey: true }, { key: 'F5' }]) {
      const harness = createHarness();
      harness.exports.init();
      const csrf = stubCsrfFetch(harness);
      let ioCalls = 0;
      harness.context.io = () => {
        ioCalls += 1;
        return harness.mockIo;
      };

      let prevented = false;
      harness.document.fire('keydown', {
        preventDefault: () => {
          prevented = true;
        },
        ...init
      });
      await flushPromises();
      await flushPromises();

      assert.ok(prevented, `${init.key} must prevent the browser default`);
      assert.equal(csrf.csrf, 1, `${init.key} must re-fetch CSRF`);
      assert.equal(ioCalls, 1, `${init.key} must reconnect Socket.IO`);
    }
  });

  test('VAL-ZOOM-002: index.html exposes #hard-refresh-btn', () => {
    assert.match(readIndexHtml(), /id="hard-refresh-btn"/, 'topbar must expose the hard refresh button');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-CONN-003 hardening: outbox durability                          */
/* ------------------------------------------------------------------ */

/* Fetch stub for the outbox: records every message_async body and can hold
 * the response open so a test can act while a drain is in flight. */
function stubOutboxFetch(harness, options = {}) {
  const sent = [];
  const gates = [];
  let released = false;
  harness.context.fetch = async (url, requestOptions) => {
    if (String(url).includes('/proxy/csrf')) {
      return new Response(JSON.stringify({ token: 't' }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    }
    sent.push(JSON.parse(requestOptions.body));
    if (options.hold && !released) {
      await new Promise((resolve) => gates.push(resolve));
    }
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { 'content-type': 'application/json' }
    });
  };
  return {
    sent,
    release: () => {
      released = true;
      gates.splice(0).forEach((resolve) => resolve());
    }
  };
}

async function settle(times = 8) {
  for (let i = 0; i < times; i += 1) {
    await flushPromises();
  }
}

describe('ui/chat.js offline outbox durability', () => {
  test('VAL-CONN-003: overlapping flushOutbox calls send each queued entry exactly once', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, flushOutbox } = harness.exports;
    state.online = true;
    harness.store['aqvn_outbox'] = JSON.stringify([
      { text: 'first', context: 'ctx-1', message_id: 'm-1', queued_at: 1 },
      { text: 'second', context: 'ctx-1', message_id: 'm-2', queued_at: 2 }
    ]);
    const net = stubOutboxFetch(harness, { hold: true });

    const first = flushOutbox();
    const second = flushOutbox();
    await settle();
    net.release();
    await Promise.all([first, second]);
    await settle();

    assert.equal(net.sent.length, 2, 'two queued entries must produce exactly two sends');
    assert.deepEqual(net.sent.map((entry) => entry.text), ['first', 'second']);
  });

  test('VAL-CONN-003: a queued entry is claimed before the send so a reconnect cannot replay it', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, flushOutbox } = harness.exports;
    state.online = true;
    harness.store['aqvn_outbox'] = JSON.stringify([
      { text: 'once', context: 'ctx-1', message_id: 'm-1', queued_at: 1 }
    ]);
    const net = stubOutboxFetch(harness, { hold: true });

    const drain = flushOutbox();
    await settle();
    assert.equal(
      harness.store['aqvn_outbox'],
      undefined,
      'the claimed entry must leave storage before the send completes'
    );
    assert.equal(net.sent.length, 1, 'the claimed entry must be sent');

    const reconnect = flushOutbox();
    await settle();
    net.release();
    await Promise.all([drain, reconnect]);
    await settle();

    assert.equal(net.sent.length, 1, 'a reconnect during the drain must not resend the claimed entry');
  });

  test('VAL-CONN-003: the outbox is capped by entry count and total bytes', () => {
    const harness = createHarness();
    harness.exports.init();
    const { queueOutbox, readOutbox, OUTBOX_MAX_ENTRIES, OUTBOX_MAX_BYTES } = harness.exports;

    const overflow = OUTBOX_MAX_ENTRIES + 20;
    for (let i = 0; i < overflow; i += 1) {
      queueOutbox('message ' + i, 'id-' + i);
    }
    const queued = readOutbox();
    assert.ok(
      queued.length <= OUTBOX_MAX_ENTRIES,
      `outbox must stay within ${OUTBOX_MAX_ENTRIES} entries, got ${queued.length}`
    );
    assert.equal(
      queued[queued.length - 1].text,
      'message ' + (overflow - 1),
      'the newest entry must be kept'
    );

    const oversized = queueOutbox('x'.repeat(OUTBOX_MAX_BYTES + 16), 'too-big');
    assert.equal(oversized.stored, false, 'an entry larger than the byte cap must be rejected');
    assert.equal(
      readOutbox().length,
      OUTBOX_MAX_ENTRIES,
      'the rejected entry must not be stored'
    );
  });

  test('VAL-CONN-003: a throwing localStorage surfaces an error and the row stops claiming delivery', async () => {
    const harness = createHarness({ storageThrows: true });
    harness.exports.init();
    const { state, sendMessage, dom } = harness.exports;
    state.online = false;
    state.polling = false;

    dom.input.value = 'cannot be stored';
    await sendMessage({ preventDefault: () => {} });

    const banner = harness.getEl('chat-error');
    assert.match(
      banner.textContent,
      /storage|queue/i,
      'a storage failure must be surfaced to the user'
    );
    assert.equal(banner.hidden, false, 'the error banner must be visible');

    let record = null;
    state.logs.forEach((entry) => {
      if (entry.content === 'cannot be stored') {
        record = entry;
      }
    });
    assert.ok(record, 'the optimistic row must still be visible');
    assert.notEqual(record.queued, true, 'an unpersisted message must not claim a queued delivery');

    const row = state.rows.get(record.id);
    assert.doesNotMatch(row.firstChild.textContent, /queued/i, 'the row must not promise delivery');
    assert.match(row.firstChild.textContent, /not sent/i, 'the row must state that nothing was sent');
  });
});
