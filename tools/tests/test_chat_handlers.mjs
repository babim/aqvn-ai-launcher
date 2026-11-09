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

function createDocumentFragmentMock() {
  const children = [];
  return {
    nodeType: 11,
    children,
    get childElementCount() {
      return children.length;
    },
    get firstChild() {
      return children[0] || null;
    },
    get lastChild() {
      return children[children.length - 1] || null;
    },
    appendChild: (child) => {
      children.push(child);
      return child;
    },
    append: (...nodes) => {
      nodes.forEach((node) => children.push(node));
    }
  };
}

function createElementMock(id) {
  const listeners = {};
  const attributes = {};
  const element = {
    id,
    nodeType: 1,
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
    click: () => {
      element.dispatch('click', {});
    },
    appendChild: (child) => {
      if (child && child.nodeType === 11) {
        while (child.children.length > 0) {
          const c = child.children.shift();
          element.children.push(c);
          if (c && typeof c === 'object') c.parentNode = element;
        }
        element.childElementCount = element.children.length;
        return child;
      }
      element.children.push(child);
      element.childElementCount = element.children.length;
      if (child && typeof child === 'object') child.parentNode = element;
      return child;
    },
    removeChild: (child) => {
      const idx = element.children.indexOf(child);
      if (idx !== -1) {
        element.children.splice(idx, 1);
        element.childElementCount = element.children.length;
        if (child && typeof child === 'object') child.parentNode = null;
      }
      return child;
    },
    insertBefore: (newNode, referenceNode) => {
      if (newNode && newNode.nodeType !== 11) {
        const existingIdx = element.children.indexOf(newNode);
        if (existingIdx !== -1) {
          element.children.splice(existingIdx, 1);
        }
      }
      if (!referenceNode) {
        return element.appendChild(newNode);
      }
      const idx = element.children.indexOf(referenceNode);
      if (idx === -1) {
        return element.appendChild(newNode);
      }
      if (newNode && newNode.nodeType === 11) {
        let insertIdx = idx;
        while (newNode.children.length > 0) {
          const c = newNode.children.shift();
          element.children.splice(insertIdx, 0, c);
          if (c && typeof c === 'object') c.parentNode = element;
          insertIdx += 1;
        }
        element.childElementCount = element.children.length;
        return newNode;
      }
      element.children.splice(idx, 0, newNode);
      element.childElementCount = element.children.length;
      if (newNode && typeof newNode === 'object') newNode.parentNode = element;
      return newNode;
    },
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
    'reloadCanvas', 'writeOutbox', 'OUTBOX_MAX_ENTRIES', 'OUTBOX_MAX_BYTES',
    'showChat', 'cycleTheme', 'applyTheme', 'readTheme', 'THEMES',
    'openSearch', 'closeSearch', 'toggleSearch', 'setSearchCategory',
    'applySearchAndFilter', 'searchNext', 'searchPrev', 'matchesCategory',
    'upsertLog', 'mergeLogs', 'placeRow', 'clearLogs', 'exportMarkdown', 'exportJson', 'copyTranscript',
    'openExportMenu', 'closeExportMenu', 'toggleExportMenu',
    'switchSidebarTab', 'loadWorkspaceFiles', 'renderWorkspaceFiles', 'uploadWorkspaceFiles', 'uploadSingleFile', 'isBinaryFile', 'formatFileSize', 'formatFileTimestamp',
    'toggleTerminal', 'setTerminalCollapsed', 'appendTerminalOutput', 'clearTerminalOutput', 'sendTerminalCommand',
    'initXtermTerminal', 'fitTerminal', 'emitTerminalResize', 'getXtermTheme', 'XTERM_THEMES',
    'initCodeMirrorEditor', 'initCodeMirrorDiffViewer', 'getCodeMirrorTheme', 'CODEMIRROR_THEMES',
    'openFilePreview', 'closeFilePreview', 'toggleFileEditor', 'saveFileEditor', 'openDiffViewer', 'closeDiffViewer',
    'manualReconnect'
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
    const item = { id, fn, delay };
    timeouts.push(item);
    if (typeof delay === 'number' && delay > 0 && delay <= 250) {
      setTimeout(() => {
        const idx = timeouts.indexOf(item);
        if (idx !== -1) {
          timeouts.splice(idx, 1);
          try {
            fn();
          } catch (_) {}
        }
      }, Math.min(delay, 5));
    }
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
  const bodyMock = createElementMock('body');
  const docElement = createElementMock('html');
  const documentMock = {
    title: '',
    readyState: 'loading',
    documentElement: docElement,
    body: bodyMock,
    getElementById: getEl,
    createElement: () => createElementMock('el'),
    createDocumentFragment: () => createDocumentFragmentMock(),
    createTextNode: () => ({}),
    hasFocus: () => (options.hasFocus !== undefined ? options.hasFocus : true),
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
    Blob,
    btoa: typeof btoa !== 'undefined' ? btoa : undefined,
    atob: typeof atob !== 'undefined' ? atob : undefined,
    TextDecoder: typeof TextDecoder !== 'undefined' ? TextDecoder : undefined,
    TextEncoder: typeof TextEncoder !== 'undefined' ? TextEncoder : undefined,
    URL,
    Headers,
    Response,
    URLSearchParams,
    document: documentMock,
    window: {
      location: { origin: 'http://localhost:3000', search: '' },
      localStorage: localStorageMock,
      setTimeout: fakeSetTimeout,
      clearTimeout: fakeClearTimeout,
      URL,
      Blob,
      btoa: typeof btoa !== 'undefined' ? btoa : undefined,
      atob: typeof atob !== 'undefined' ? atob : undefined,
      TextDecoder: typeof TextDecoder !== 'undefined' ? TextDecoder : undefined,
      TextEncoder: typeof TextEncoder !== 'undefined' ? TextEncoder : undefined,
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
    'password', 'username', 'input', 'status', 'stop', 'theme-toggle-btn',
    'search-toggle-btn', 'search-drawer', 'search-input', 'search-count',
    'search-prev-btn', 'search-next-btn', 'search-close-btn', 'search-filters',
    'filter-all-btn', 'filter-code-btn', 'filter-tool-btn', 'filter-thought-btn',
    'filter-error-btn', 'messages', 'export-toggle-btn', 'export-menu',
    'export-md-btn', 'export-json-btn', 'copy-transcript-btn', 'connection-retry-btn',
    'tab-sessions-btn', 'tab-workspace-btn', 'sessions-view', 'workspace-view',
    'workspace-file-list', 'workspace-upload-dropzone', 'workspace-refresh-btn',
    'terminal-drawer', 'toggle-terminal-btn', 'terminal-output', 'terminal-input',
    'terminal-send-btn', 'terminal-clear-btn', 'terminal-close-btn',
    'file-preview-modal', 'file-preview-title', 'file-preview-content', 'file-editor-textarea',
    'file-code-editor', 'file-preview-edit-btn', 'file-preview-save-btn', 'close-file-preview-btn',
    'diff-viewer-modal', 'diff-viewer-title', 'close-diff-viewer-btn', 'diff-viewer-content', 'diff-code-editor'
  ].forEach((id) => {
    dom[id] = getEl(id);
  });
  dom.retryBtn = getEl('connection-retry-btn');
  dom.retryBtn.hidden = true;
  dom.chatError = getEl('chat-error');
  dom.chatError.hidden = true;
  dom.terminalDrawer = getEl('terminal-drawer');
  dom.terminalDrawer.hidden = true;
  dom.toggleTerminalBtn = getEl('toggle-terminal-btn');
  dom.terminalOutput = getEl('terminal-output');
  dom.terminalInput = getEl('terminal-input');
  dom.terminalSendBtn = getEl('terminal-send-btn');
  dom.terminalClearBtn = getEl('terminal-clear-btn');
  dom.terminalCloseBtn = getEl('terminal-close-btn');
  dom.tabSessionsBtn = getEl('tab-sessions-btn');
  dom.tabSessionsBtn.classList.add('active');
  dom.tabWorkspaceBtn = getEl('tab-workspace-btn');
  dom.sessionsView = getEl('sessions-view');
  dom.sessionsView.hidden = false;
  dom.workspaceView = getEl('workspace-view');
  dom.workspaceView.hidden = true;
  dom.workspaceFileList = getEl('workspace-file-list');
  dom.workspaceUploadDropzone = getEl('workspace-upload-dropzone');
  dom.workspaceRefreshBtn = getEl('workspace-refresh-btn');
  dom.composer = getEl('composer');
  dom.themeToggleBtn = getEl('theme-toggle-btn');
  dom.searchToggleBtn = getEl('search-toggle-btn');
  dom.searchDrawer = getEl('search-drawer');
  dom.searchInput = getEl('search-input');
  dom.searchCount = getEl('search-count');
  dom.searchPrevBtn = getEl('search-prev-btn');
  dom.searchNextBtn = getEl('search-next-btn');
  dom.searchCloseBtn = getEl('search-close-btn');
  dom.searchFilters = getEl('search-filters');
  dom.filterAllBtn = getEl('filter-all-btn');
  dom.filterAllBtn.dataset.category = 'all';
  dom.filterCodeBtn = getEl('filter-code-btn');
  dom.filterCodeBtn.dataset.category = 'code_exe';
  dom.filterToolBtn = getEl('filter-tool-btn');
  dom.filterToolBtn.dataset.category = 'tool';
  dom.filterThoughtBtn = getEl('filter-thought-btn');
  dom.filterThoughtBtn.dataset.category = 'thought';
  dom.filterErrorBtn = getEl('filter-error-btn');
  dom.filterErrorBtn.dataset.category = 'error';
  dom.messages = getEl('messages');
  dom.exportToggleBtn = getEl('export-toggle-btn');
  dom.exportMenu = getEl('export-menu');
  dom.exportMdBtn = getEl('export-md-btn');
  dom.exportJsonBtn = getEl('export-json-btn');
  dom.copyTranscriptBtn = getEl('copy-transcript-btn');

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

  test('VAL-UX-001: notify_task_complete fires when window is visible but unfocused (!document.hasFocus())', () => {
    const harness = createHarness();
    const calls = tauriRecorder(harness);
    const { applyProgress } = harness.exports;
    const notes = () => calls.filter((c) => c.command === 'notify_task_complete');

    // Visible and focused: no notification
    harness.document.hidden = false;
    harness.document.hasFocus = () => true;
    applyProgress(true, 'working');
    applyProgress(false, 'done');
    assert.equal(notes().length, 0, 'visible and focused window must not raise a notification');

    // Visible but unfocused: raises notification
    harness.document.hidden = false;
    harness.document.hasFocus = () => false;
    applyProgress(true, 'background job');
    applyProgress(false, 'background done');
    assert.equal(notes().length, 1, 'unfocused window must raise notification');
    assert.equal(notes()[0].args.title, 'Task complete');
    assert.match(notes()[0].args.body, /background done/);
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

  test('VAL-UI-ZOOM-001: Single-Point Zoom Normalization - chat.js defers to documentElement without mutating body when container bridge is active', () => {
    const harness = createHarness();
    let bridgeApplyCalls = [];
    let bridgeZoom = 1.0;
    harness.context.window.__AQVN_CONTAINER__ = {
      applyZoom: (lvl) => {
        bridgeApplyCalls.push(lvl);
        bridgeZoom = typeof lvl === 'number' ? lvl : parseFloat(lvl);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      readZoom: () => bridgeZoom,
      zoomIn: () => {
        bridgeZoom = Math.min(2.0, Math.round((bridgeZoom + 0.1) * 100) / 100);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      zoomOut: () => {
        bridgeZoom = Math.max(0.7, Math.round((bridgeZoom - 0.1) * 100) / 100);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      resetZoom: () => {
        bridgeZoom = 1.0;
        harness.document.documentElement.style.zoom = '1';
        return 1.0;
      }
    };

    harness.exports.init();

    // 1. Initial state: body.style.zoom must NOT be applied; documentElement is used
    assert.equal(harness.document.body.style.zoom, undefined, 'ui/chat.js must not apply document.body.style.zoom if container bridge is detected');
    assert.equal(harness.document.documentElement.style.zoom, '1', 'documentElement.style.zoom initialized');

    const key = (init) => Object.assign({ preventDefault: () => {} }, init);

    // 2. Ctrl + scales view by exactly 10% step (1.0 -> 1.1) on documentElement, body untouched
    harness.document.fire('keydown', key({ key: '=', ctrlKey: true }));
    assert.equal(harness.document.documentElement.style.zoom, '1.1', 'Ctrl + must scale documentElement by exactly 10% step');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom must not be mutated on zoom in');

    // 3. Second Ctrl + scales view to 1.2, body untouched
    harness.document.fire('keydown', key({ key: '+', ctrlKey: true }));
    assert.equal(harness.document.documentElement.style.zoom, '1.2', 'second Ctrl + must scale documentElement to 1.2');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom must not be mutated');

    // 4. Ctrl - scales view down by 10% step (1.2 -> 1.1), body untouched
    harness.document.fire('keydown', key({ key: '-', ctrlKey: true }));
    assert.equal(harness.document.documentElement.style.zoom, '1.1', 'Ctrl - must scale documentElement down by 10% step');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom must not be mutated');

    // 5. Ctrl 0 resets zoom to 1.0 on documentElement, body untouched
    harness.document.fire('keydown', key({ key: '0', ctrlKey: true }));
    assert.equal(harness.document.documentElement.style.zoom, '1', 'Ctrl 0 must reset documentElement to 1.0');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom must not be mutated');

    // 6. Direct applyZoom call also defers to bridge and avoids mutating body
    harness.exports.applyZoom(1.35);
    assert.equal(harness.document.documentElement.style.zoom, '1.35', 'applyZoom updates documentElement via bridge');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom must not be mutated by direct applyZoom');

    // 7. Event already handled by container bridge (defaultPrevented or __aqvn_zoom_handled) is dropped to prevent double multiplication
    const handledEvent = key({ key: '=', ctrlKey: true, defaultPrevented: true });
    harness.document.fire('keydown', handledEvent);
    assert.equal(harness.document.documentElement.style.zoom, '1.35', 'already-handled event must not increment zoom again');

    const markedEvent = key({ key: '=', ctrlKey: true, __aqvn_zoom_handled: true });
    harness.document.fire('keydown', markedEvent);
    assert.equal(harness.document.documentElement.style.zoom, '1.35', 'marked event must not increment zoom again');
  });

  test('VAL-UI-ZOOM-001: stored aqvn_zoom with active container bridge applies to documentElement only', () => {
    const harness = createHarness();
    harness.store['aqvn_zoom'] = '1.4';
    let bridgeZoom = 1.4;
    harness.context.window.__AQVN_CONTAINER__ = {
      applyZoom: (lvl) => {
        bridgeZoom = typeof lvl === 'number' ? lvl : parseFloat(lvl);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      readZoom: () => bridgeZoom,
      zoomIn: () => {
        bridgeZoom = Math.min(2.0, Math.round((bridgeZoom + 0.1) * 100) / 100);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      zoomOut: () => {
        bridgeZoom = Math.max(0.7, Math.round((bridgeZoom - 0.1) * 100) / 100);
        harness.document.documentElement.style.zoom = String(bridgeZoom);
        return bridgeZoom;
      },
      resetZoom: () => {
        bridgeZoom = 1.0;
        harness.document.documentElement.style.zoom = '1';
        return 1.0;
      }
    };
    harness.exports.init();
    assert.equal(harness.document.documentElement.style.zoom, '1.4', 'init must apply stored zoom to documentElement');
    assert.equal(harness.document.body.style.zoom, undefined, 'init must not set body.style.zoom when container bridge is present');

    const key = (init) => Object.assign({ preventDefault: () => {} }, init);
    harness.document.fire('keydown', key({ key: '-', ctrlKey: true }));
    assert.equal(harness.document.documentElement.style.zoom, '1.3', 'Ctrl - decrements stored zoom by 10%');
    assert.equal(harness.document.body.style.zoom, undefined, 'body.style.zoom remains undefined');
  });

  test('VAL-ZOOM-002: #hard-refresh-btn resets RAM state, CSRF, socket and canvas iframe', async () => {
    const harness = createHarness();
    harness.exports.init();
    harness.getEl('login-view').hidden = true;
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
      harness.getEl('login-view').hidden = true;
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

/* ------------------------------------------------------------------ */
/* Edge cases: outbox fallback, canvas reload, server sync, LED,      */
/* and hard-refresh login guard                                       */
/* ------------------------------------------------------------------ */

describe('ui/chat.js edge cases and connection state hygiene', () => {
  test('sendMessage falls back to outbox when apiCall(message_async) rejects even if state.online is true', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, sendMessage, dom } = harness.exports;
    state.online = true;
    state.polling = false;

    harness.context.fetch = async (url) => {
      if (String(url).includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ token: 't' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: false, error: 'Network failure' }), {
        status: 500,
        headers: { 'content-type': 'application/json' }
      });
    };

    dom.input.value = 'message during drop';
    await sendMessage({ preventDefault: () => {} });

    const raw = harness.store['aqvn_outbox'];
    assert.ok(raw, 'failed send must fall back to aqvn_outbox even if state.online is true');
    const queued = JSON.parse(raw);
    assert.equal(queued.length, 1);
    assert.equal(queued[0].text, 'message during drop');

    const id = queued[0].message_id;
    const record = state.logs.get(id);
    assert.ok(record);
    assert.equal(record.queued, true, 'record must be marked queued');
    assert.equal(record.failed, false, 'record must not be marked failed');

    const row = state.rows.get(id);
    assert.ok(row);
    assert.match(row.className, /msg-queued/, 'row must have msg-queued class');
  });

  test('stopPolling restores online connection pill if state.online === true', () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, stopPolling, setConnection } = harness.exports;
    const pill = harness.getEl('connection-pill');

    state.online = true;
    setConnection('polling');
    assert.equal(pill.dataset.state, 'polling', 'pill should initially be in polling state');

    stopPolling();
    assert.equal(pill.dataset.state, 'online', 'stopPolling must restore online state when state.online is true');
  });

  test('hardRefresh on login view skips connectSocket', async () => {
    const harness = createHarness();
    harness.exports.init();
    const csrf = stubCsrfFetch(harness);
    const { hardRefresh, dom } = harness.exports;

    dom.loginView.hidden = false;
    let ioCalls = 0;
    harness.context.io = () => {
      ioCalls += 1;
      return harness.mockIo;
    };

    const canvas = harness.getEl('canvas-frame');
    canvas.removeAttribute('src');

    await hardRefresh();
    await flushPromises();

    assert.equal(csrf.csrf, 1, 'hard refresh still refreshes CSRF on login view');
    assert.equal(ioCalls, 0, 'connectSocket must not be called when login view is visible');
    assert.equal(canvas.getAttribute('src'), null, 'reloadCanvas must not be called when login view is visible');
  });

  test('sidebar server switcher synchronizes dom.serverUrl', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;
    const sidebarSelect = harness.getEl('sidebar-server-select');
    sidebarSelect.value = 'https://custom-server.example.com';
    sidebarSelect.dispatch('change', {});

    assert.equal(dom.serverUrl.value, 'https://custom-server.example.com');
  });

  test('showChat reloads canvas iframe', () => {
    const harness = createHarness();
    harness.exports.init();
    const { showChat, state } = harness.exports;
    const canvas = harness.getEl('canvas-frame');
    canvas.removeAttribute('src');

    showChat();
    assert.equal(
      canvas.getAttribute('src'),
      state.base + '/plugins/aqvn_intel_suite/webui/canvas.html'
    );
  });
});

/* ------------------------------------------------------------------ */
/* VAL-WEB-004: Injected OS notification and title sync bridge        */
/* ------------------------------------------------------------------ */

function createContainerHarness(options = {}) {
  const filePath = new URL('../../ui/container.js', import.meta.url);
  const code = fs.readFileSync(filePath, 'utf8');

  const elements = {};
  function getEl(id) {
    return elements[id] || null;
  }

  const calls = [];
  const docListeners = {};
  const store = options.localStorage ? { ...options.localStorage } : {};
  const documentMock = {
    title: 'AQVN Workspace',
    hidden: options.hidden !== undefined ? options.hidden : false,
    hasFocus: () => (options.hasFocus !== undefined ? options.hasFocus : true),
    documentElement: {
      style: {}
    },
    body: {
      classList: createClassList(),
      getAttribute: (name) => (name in attributes ? attributes[name] : null),
      children: [],
      style: {}
    },
    getElementById: getEl,
    querySelector: (selector) => {
      if (selector === '[aria-busy="true"]') {
        return options.ariaBusy ? createElementMock('busy-el') : null;
      }
      if (selector.includes('stop')) {
        return elements['stop'] || null;
      }
      if (selector.includes('status')) {
        return elements['status'] || null;
      }
      return null;
    },
    addEventListener: (type, fn) => {
      (docListeners[type] = docListeners[type] || []).push(fn);
    }
  };

  const attributes = {};
  let reloaded = false;
  const warnings = [];
  const customConsole = {
    ...console,
    warn: (msg, ...args) => {
      warnings.push([msg, ...args]);
      if (options.onWarn) options.onWarn(msg, ...args);
    }
  };

  const context = {
    console: customConsole,
    document: documentMock,
    location: {
      origin: 'http://localhost:3000',
      reload: () => {
        reloaded = true;
      }
    },
    window: {
      AudioContext: options.AudioContext !== undefined ? options.AudioContext : undefined,
      webkitAudioContext: options.webkitAudioContext !== undefined ? options.webkitAudioContext : undefined,
      location: {
        origin: 'http://localhost:3000',
        reload: () => {
          reloaded = true;
        }
      },
      localStorage: {
        getItem: (k) => (k in store ? store[k] : null),
        setItem: (k, v) => {
          store[k] = String(v);
        },
        removeItem: (k) => {
          delete store[k];
        }
      },
      addEventListener: (type, fn) => {
        (docListeners[type] = docListeners[type] || []).push(fn);
      },
      dispatchEvent: (event) => {
        const type = (event && event.type) || String(event);
        (docListeners[type] || []).forEach((fn) => fn(event));
        return true;
      },
      postMessage: (data) => {
        (docListeners['message'] || []).forEach((fn) => fn({ data }));
      },
      __TAURI_INTERNALS__: {
        invoke: (command, args) => {
          if (options.ipcReject) {
            return Promise.reject(new Error('Simulated IPC error for ' + command));
          }
          calls.push({ command, args });
          return Promise.resolve();
        }
      }
    }
  };

  vm.createContext(context);
  vm.runInContext(code, context);

  return {
    context,
    calls,
    warnings,
    document: documentMock,
    docListeners,
    elements,
    createElement: createElementMock,
    getReloaded: () => reloaded,
    store,
    bridge: context.window.__AQVN_CONTAINER__
  };
}

describe('VAL-WEB-004: test_web_notification_bridge container bridge script', () => {
  test('test_web_notification_bridge script exists and is readable', () => {
    const filePath = new URL('../../ui/container.js', import.meta.url);
    assert.ok(fs.existsSync(filePath), 'ui/container.js must exist');
    const content = fs.readFileSync(filePath, 'utf8');
    assert.ok(content.length > 50, 'ui/container.js must contain bridge script');
  });

  test('test_web_notification_bridge detects task completion from WebSocket state', () => {
    const harness = createContainerHarness({ hidden: true });
    const { bridge, calls } = harness;
    assert.ok(bridge, 'window.__AQVN_CONTAINER__ bridge must be exposed');

    // 1. Socket.IO log_progress active: true
    bridge.handleWebSocketMessage('42["log_progress", {"active": true, "progress": "Thinking..."}]');
    assert.equal(bridge.getState().busy, true, 'bridge must enter busy state');
    const trayBusyCalls = calls.filter((c) => c.command === 'set_tray_busy' && c.args.busy === true);
    assert.equal(trayBusyCalls.length, 1, 'set_tray_busy(true) must be dispatched');

    // 2. Socket.IO log_progress active: false while hidden
    bridge.handleWebSocketMessage('42["log_progress", {"active": false, "progress": "Analysis complete"}]');
    assert.equal(bridge.getState().busy, false, 'bridge must return to idle state');
    const trayIdleCalls = calls.filter((c) => c.command === 'set_tray_busy' && c.args.busy === false);
    assert.equal(trayIdleCalls.length, 1, 'set_tray_busy(false) must be dispatched');

    const notifyCalls = calls.filter((c) => c.command === 'notify_task_complete');
    assert.equal(notifyCalls.length, 1, 'notify_task_complete must be dispatched when hidden');
    assert.equal(notifyCalls[0].args.title, 'Task complete');
    assert.match(notifyCalls[0].args.body, /Analysis complete/);

    // 3. Socket.IO state_push snapshot
    bridge.handleWebSocketMessage('42["state_push", {"snapshot": {"log_progress_active": true, "log_progress": "Refining..."}}]');
    assert.equal(bridge.getState().busy, true);
    bridge.handleWebSocketMessage('42["state_push", {"snapshot": {"log_progress_active": false, "log_progress": "All tasks done"}}]');
    assert.equal(bridge.getState().busy, false);
    assert.equal(calls.filter((c) => c.command === 'notify_task_complete').length, 2);

    // 4. Raw JSON state
    bridge.handleWebSocketMessage(JSON.stringify({ active: true, progress: "Running query" }));
    assert.equal(bridge.getState().busy, true);
    bridge.handleWebSocketMessage(JSON.stringify({ active: false, progress: "Query finished" }));
    assert.equal(bridge.getState().busy, false);
    assert.equal(calls.filter((c) => c.command === 'notify_task_complete').length, 3);
  });

  test('test_web_notification_bridge detects task completion from DOM state', () => {
    const harness = createContainerHarness({ hidden: true });
    const { bridge, calls, document } = harness;

    // Body busy class added
    document.body.classList.add('busy');
    bridge.checkDomState();
    assert.equal(bridge.getState().busy, true, 'body.busy must trigger busy state');
    assert.equal(calls.filter((c) => c.command === 'set_tray_busy' && c.args.busy === true).length, 1);

    // Body busy class removed
    document.body.classList.remove('busy');
    bridge.checkDomState();
    assert.equal(bridge.getState().busy, false, 'body.busy removal must trigger idle state');
    assert.equal(calls.filter((c) => c.command === 'set_tray_busy' && c.args.busy === false).length, 1);
    assert.equal(calls.filter((c) => c.command === 'notify_task_complete').length, 1);
  });

  test('test_web_notification_bridge dispatches Tauri IPC notify_task_complete only when document.hidden === true', () => {
    // Visible window: hidden = false
    const visibleHarness = createContainerHarness({ hidden: false });
    visibleHarness.bridge.setTaskState(true, 'working visible');
    visibleHarness.bridge.setTaskState(false, 'done visible');
    assert.equal(
      visibleHarness.calls.filter((c) => c.command === 'notify_task_complete').length,
      0,
      'visible window must not dispatch notify_task_complete'
    );
    assert.equal(
      visibleHarness.calls.filter((c) => c.command === 'set_tray_busy').length,
      2,
      'visible window must still update tray state'
    );

    // Hidden window: hidden = true
    const hiddenHarness = createContainerHarness({ hidden: true });
    hiddenHarness.bridge.setTaskState(true, 'working hidden');
    hiddenHarness.bridge.setTaskState(false, 'done hidden');
    assert.equal(
      hiddenHarness.calls.filter((c) => c.command === 'notify_task_complete').length,
      1,
      'hidden window must dispatch notify_task_complete'
    );
    assert.equal(hiddenHarness.calls.find((c) => c.command === 'notify_task_complete').args.title, 'Task complete');
    assert.match(hiddenHarness.calls.find((c) => c.command === 'notify_task_complete').args.body, /done hidden/);
  });

  test('VAL-UX-001: test_web_notification_bridge triggers notification when window is visible but unfocused', () => {
    const harness = createContainerHarness({ hidden: false, hasFocus: false });
    const { bridge, calls } = harness;

    bridge.setTaskState(true, 'working unfocused');
    bridge.setTaskState(false, 'done unfocused');
    const notifyCalls = calls.filter((c) => c.command === 'notify_task_complete');
    assert.equal(notifyCalls.length, 1, 'visible but unfocused window must trigger notification');
    assert.equal(notifyCalls[0].args.title, 'Task complete');
    assert.match(notifyCalls[0].args.body, /done unfocused/);
  });

  test('VAL-UX-002: ui/container.js synthesizes dual-tone Web Audio chime on task complete when enabled', () => {
    const audioEvents = [];
    class MockAudioContext {
      constructor() {
        this.currentTime = 0;
        this.destination = {};
        audioEvents.push({ type: 'create_context' });
      }
      createOscillator() {
        const osc = {
          type: '',
          frequency: {
            setValueAtTime: (val, time) => {
              audioEvents.push({ type: 'freq', val, time });
            }
          },
          connect: (dest) => {
            audioEvents.push({ type: 'osc_connect', dest });
          },
          start: (time) => {
            audioEvents.push({ type: 'osc_start', time });
          },
          stop: (time) => {
            audioEvents.push({ type: 'osc_stop', time });
          }
        };
        return osc;
      }
      createGain() {
        const gain = {
          gain: {
            setValueAtTime: (val, time) => {
              audioEvents.push({ type: 'gain_set', val, time });
            },
            exponentialRampToValueAtTime: (val, time) => {
              audioEvents.push({ type: 'gain_ramp', val, time });
            }
          },
          connect: (dest) => {
            audioEvents.push({ type: 'gain_connect', dest });
          }
        };
        return gain;
      }
      close() {
        audioEvents.push({ type: 'close_context' });
        return Promise.resolve();
      }
    }

    // Enabled (default: true): plays chime
    const enabledHarness = createContainerHarness({
      AudioContext: MockAudioContext
    });
    assert.equal(typeof enabledHarness.bridge.isTaskChimeEnabled, 'function', 'isTaskChimeEnabled must be exposed');
    assert.equal(enabledHarness.bridge.isTaskChimeEnabled(), true, 'task chime must default to enabled');

    enabledHarness.bridge.setTaskState(true, 'working');
    assert.equal(audioEvents.length, 0, 'no sound while task is running');

    enabledHarness.bridge.setTaskState(false, 'done');
    assert.ok(audioEvents.length > 0, 'audio events must fire on task complete');

    const freqEvents = audioEvents.filter((e) => e.type === 'freq');
    assert.equal(freqEvents.length, 2, 'must schedule dual frequencies');
    assert.equal(freqEvents[0].val, 440, 'tone 1 must be 440Hz');
    assert.equal(freqEvents[1].val, 880, 'tone 2 must be 880Hz');
    assert.ok(freqEvents[1].time > freqEvents[0].time, 'tone 2 must be scheduled after tone 1');

    const oscStop = audioEvents.find((e) => e.type === 'osc_stop');
    assert.ok(oscStop, 'oscillator stop must be scheduled');
    assert.equal(oscStop.time, 0.2, 'total tone duration must be 200ms (0.2s)');

    // Disabled (task_chime_enabled: false): does not play chime
    audioEvents.length = 0;
    const disabledHarness = createContainerHarness({
      AudioContext: MockAudioContext,
      localStorage: { aqvn_task_chime_enabled: 'false' }
    });
    assert.equal(disabledHarness.bridge.isTaskChimeEnabled(), false, 'task chime disabled when storage is false');

    disabledHarness.bridge.setTaskState(true, 'working');
    disabledHarness.bridge.setTaskState(false, 'done');
    assert.equal(audioEvents.length, 0, 'no sound should play when task_chime_enabled is false');
  });

  test('VAL-UX-002: ui/container.js respects task_chime_volume and task_chime_tone customization', () => {
    const audioEvents = [];
    class MockAudioContext {
      constructor() {
        this.currentTime = 0;
        this.destination = {};
        audioEvents.push({ type: 'create_context' });
      }
      createOscillator() {
        return {
          type: '',
          frequency: {
            setValueAtTime: (val, time) => {
              audioEvents.push({ type: 'freq', val, time });
            }
          },
          connect: (dest) => audioEvents.push({ type: 'osc_connect', dest }),
          start: (time) => audioEvents.push({ type: 'osc_start', time }),
          stop: (time) => audioEvents.push({ type: 'osc_stop', time })
        };
      }
      createGain() {
        return {
          gain: {
            setValueAtTime: (val, time) => audioEvents.push({ type: 'gain_set', val, time }),
            exponentialRampToValueAtTime: (val, time) => audioEvents.push({ type: 'gain_ramp', val, time }),
            linearRampToValueAtTime: (val, time) => audioEvents.push({ type: 'gain_ramp', val, time })
          },
          connect: (dest) => audioEvents.push({ type: 'gain_connect', dest })
        };
      }
      close() {
        audioEvents.push({ type: 'close_context' });
        return Promise.resolve();
      }
    }

    const harness = createContainerHarness({
      AudioContext: MockAudioContext,
      localStorage: {
        aqvn_task_chime_enabled: 'true',
        aqvn_task_chime_volume: '0.8',
        aqvn_task_chime_tone: 'bell'
      }
    });

    assert.equal(typeof harness.bridge.getTaskChimeVolume, 'function', 'getTaskChimeVolume must be function');
    assert.equal(typeof harness.bridge.setTaskChimeVolume, 'function', 'setTaskChimeVolume must be function');
    assert.equal(typeof harness.bridge.getTaskChimeTone, 'function', 'getTaskChimeTone must be function');
    assert.equal(typeof harness.bridge.setTaskChimeTone, 'function', 'setTaskChimeTone must be function');

    assert.equal(harness.bridge.getTaskChimeVolume(), 0.8, 'volume loaded from storage');
    assert.equal(harness.bridge.getTaskChimeTone(), 'bell', 'tone loaded from storage');

    harness.bridge.setTaskState(true, 'running');
    harness.bridge.setTaskState(false, 'finished');

    const freqs = audioEvents.filter((e) => e.type === 'freq');
    assert.ok(freqs.length >= 2, 'bell tone must schedule frequencies');
    assert.equal(freqs[0].val, 587.33, 'bell tone starts at 587.33Hz');
    assert.equal(freqs[1].val, 880, 'bell tone second freq is 880Hz');

    const gainSets = audioEvents.filter((e) => e.type === 'gain_set');
    assert.ok(gainSets.length > 0, 'gain must be set');
    assert.ok(Math.abs(gainSets[0].val - 0.16) < 0.001, 'gain must scale with 0.8 volume (0.2 * 0.8 = 0.16)');

    // Test tone switching via API
    audioEvents.length = 0;
    harness.bridge.setTaskChimeTone('beep');
    harness.bridge.setTaskChimeVolume(1.0);
    harness.bridge.setTaskState(true, 'running');
    harness.bridge.setTaskState(false, 'finished');
    const beepFreqs = audioEvents.filter((e) => e.type === 'freq');
    assert.equal(beepFreqs[0].val, 800, 'beep tone starts at 800Hz');
  });

  test('test_web_notification_bridge updates system tray icon state between idle and busy', () => {
    const harness = createContainerHarness({ hidden: false });
    const { bridge, calls } = harness;

    bridge.setTaskState(true);
    const lastBusy = calls[calls.length - 1];
    assert.equal(lastBusy.command, 'set_tray_busy');
    assert.equal(lastBusy.args.busy, true);

    bridge.setTaskState(false);
    const lastIdle = calls[calls.length - 1];
    assert.equal(lastIdle.command, 'set_tray_busy');
    assert.equal(lastIdle.args.busy, false);
  });

  test('test_web_notification_bridge init_script carries container bridge script', () => {
    const rsPath = new URL('../../src-tauri/src/proxy/server.rs', import.meta.url);
    const rsContent = fs.readFileSync(rsPath, 'utf8');
    assert.match(
      rsContent,
      /CONTAINER_BRIDGE_SCRIPT|ui\/container\.js/,
      'src-tauri/src/proxy/server.rs must reference container bridge script'
    );
  });

  test('test_web_notification_bridge dispatches open_settings on Ctrl+, / Cmd+, keydown', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    let prevented = false;
    const event = {
      ctrlKey: true,
      metaKey: false,
      key: ',',
      preventDefault: () => {
        prevented = true;
      }
    };
    const keydownListeners = docListeners['keydown'] || [];
    assert.ok(keydownListeners.length > 0, 'keydown listener must be registered');
    for (const fn of keydownListeners) {
      fn(event);
    }
    assert.equal(prevented, true, 'Ctrl+, must call preventDefault()');
    const openSettingsCalls = calls.filter((c) => c.command === 'open_settings');
    assert.equal(openSettingsCalls.length, 1, 'open_settings IPC command must be dispatched');
  });

  test('VAL-WIN-001: applyZoom sets documentElement.style.zoom only and accepts absolute target scale', () => {
    const harness = createContainerHarness();
    const { document, bridge, store } = harness;

    const result = bridge.applyZoom(1.35);
    assert.equal(result, 1.35);
    assert.equal(document.documentElement.style.zoom, '1.35');
    assert.equal(document.body.style.zoom, undefined, 'body.style.zoom must not be mutated');
    assert.equal(store['aqvn_zoom'], '1.35');

    bridge.applyZoom('0.85');
    assert.equal(document.documentElement.style.zoom, '0.85');
    assert.equal(document.body.style.zoom, undefined);
    assert.equal(store['aqvn_zoom'], '0.85');

    bridge.applyZoom({ scale: 1.5 });
    assert.equal(document.documentElement.style.zoom, '1.5');
    assert.equal(document.body.style.zoom, undefined);
    assert.equal(store['aqvn_zoom'], '1.5');

    bridge.applyZoom(3.5);
    assert.equal(document.documentElement.style.zoom, '2');
    bridge.applyZoom(0.2);
    assert.equal(document.documentElement.style.zoom, '0.7');
  });

  test('VAL-WIN-001: keyHandler drops repeated zoom shortcut events within 150ms debounce window', () => {
    const harness = createContainerHarness();
    const { document, docListeners } = harness;
    const keydownListeners = docListeners['keydown'] || [];

    function fireKeyAt(key, timeStamp, ctrlKey = true) {
      let prevented = false;
      const event = {
        ctrlKey,
        metaKey: false,
        shiftKey: false,
        altKey: false,
        key,
        timeStamp,
        preventDefault: () => {
          prevented = true;
        }
      };
      for (const fn of keydownListeners) {
        fn(event);
      }
      return prevented;
    }

    // First zoom in at t=1000ms: accepted -> 1.1
    assert.equal(fireKeyAt('=', 1000), true);
    assert.equal(document.documentElement.style.zoom, '1.1');
    assert.equal(document.body.style.zoom, undefined);

    // Repeated zoom in at t=1050ms (< 150ms): dropped -> zoom stays 1.1
    assert.equal(fireKeyAt('=', 1050), true);
    assert.equal(document.documentElement.style.zoom, '1.1');

    // Zoom in at t=1160ms (>= 150ms): accepted -> 1.2
    assert.equal(fireKeyAt('+', 1160), true);
    assert.equal(document.documentElement.style.zoom, '1.2');
  });

  test('test_web_notification_bridge handles zoom shortcuts (Ctrl+=, Ctrl+-, Ctrl+0) and sets documentElement.style.zoom without mutating body.style.zoom', () => {
    const harness = createContainerHarness();
    const { document, docListeners, store } = harness;
    const keydownListeners = docListeners['keydown'] || [];
    assert.ok(keydownListeners.length > 0, 'keydown listener must be registered');

    let simulatedTime = 1000;
    function fireKey(key, ctrlKey = true, extra = {}) {
      let prevented = false;
      const event = {
        ctrlKey,
        metaKey: false,
        shiftKey: false,
        altKey: false,
        key,
        timeStamp: simulatedTime,
        preventDefault: () => {
          prevented = true;
        },
        ...extra
      };
      simulatedTime += 200;
      for (const fn of keydownListeners) {
        fn(event);
      }
      return prevented;
    }

    // Zoom in: Ctrl+=
    assert.equal(fireKey('='), true);
    assert.equal(document.documentElement.style.zoom, '1.1');
    assert.equal(document.body.style.zoom, undefined, 'body.style.zoom must not be mutated');
    assert.equal(store['aqvn_zoom'], '1.1');

    // Zoom in again: Ctrl++
    assert.equal(fireKey('+'), true);
    assert.equal(document.documentElement.style.zoom, '1.2');
    assert.equal(document.body.style.zoom, undefined);

    // Zoom out: Ctrl+-
    assert.equal(fireKey('-'), true);
    assert.equal(document.documentElement.style.zoom, '1.1');

    // Reset zoom: Ctrl+0
    assert.equal(fireKey('0'), true);
    assert.equal(document.documentElement.style.zoom, '1');
    assert.equal(store['aqvn_zoom'], '1');

    // Zoom in with Shift (Ctrl+Shift+= on standard keyboard)
    assert.equal(fireKey('+', true, { shiftKey: true }), true);
    assert.equal(document.documentElement.style.zoom, '1.1');

    // Zoom out with Ctrl+_ (Shift+Minus)
    assert.equal(fireKey('_', true, { shiftKey: true, code: 'Minus' }), true);
    assert.equal(document.documentElement.style.zoom, '1');

    // Zoom in with NumpadAdd
    assert.equal(fireKey('Add', true, { code: 'NumpadAdd' }), true);
    assert.equal(document.documentElement.style.zoom, '1.1');

    // Zoom out with NumpadSubtract
    assert.equal(fireKey('Subtract', true, { code: 'NumpadSubtract' }), true);
    assert.equal(document.documentElement.style.zoom, '1');

    // Zoom in, then reset with Numpad0
    assert.equal(fireKey('+', true), true);
    assert.equal(document.documentElement.style.zoom, '1.1');
    assert.equal(fireKey('0', true, { code: 'Numpad0' }), true);
    assert.equal(document.documentElement.style.zoom, '1');

    // Native Tauri webview zoom calls must be dispatched alongside CSS zoom
    const nativeZoomCalls = harness.calls.filter(
      (c) => c.command === 'plugin:webview|set_webview_zoom'
    );
    assert.ok(
      nativeZoomCalls.length > 0,
      'Native plugin:webview|set_webview_zoom IPC must be invoked on zoom'
    );
  });

  test('test_web_notification_bridge handles hard refresh shortcuts (F5, Ctrl+Shift+R)', () => {
    const harness = createContainerHarness();
    const { docListeners, getReloaded } = harness;
    const keydownListeners = docListeners['keydown'] || [];

    function fireKey(key, ctrlKey = false, shiftKey = false) {
      let prevented = false;
      const event = {
        ctrlKey,
        metaKey: false,
        shiftKey,
        altKey: false,
        key,
        preventDefault: () => {
          prevented = true;
        }
      };
      for (const fn of keydownListeners) {
        fn(event);
      }
      return prevented;
    }

    // F5
    assert.equal(fireKey('F5'), true);
    assert.equal(getReloaded(), true);

    // Ctrl+Shift+R
    assert.equal(fireKey('R', true, true), true);
  });

  test('test_web_notification_bridge dispatches show_context_menu on contextmenu when no text selected', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const contextMenuListeners = docListeners['contextmenu'] || [];
    assert.ok(contextMenuListeners.length > 0, 'contextmenu listener must be registered');

    let prevented = false;
    const event = {
      preventDefault: () => {
        prevented = true;
      }
    };
    for (const fn of contextMenuListeners) {
      fn(event);
    }
    assert.equal(prevented, true, 'contextmenu without selection must call preventDefault()');
    const ctxCalls = calls.filter((c) => c.command === 'show_context_menu');
    assert.equal(ctxCalls.length, 1, 'show_context_menu IPC command must be dispatched');
  });

  test('test_web_notification_bridge allows native context menu when text is selected', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const contextMenuListeners = docListeners['contextmenu'] || [];

    // Mock active text selection on window
    harness.context.window.getSelection = () => ({
      toString: () => 'selected text snippet'
    });

    let prevented = false;
    const event = {
      preventDefault: () => {
        prevented = true;
      }
    };
    for (const fn of contextMenuListeners) {
      fn(event);
    }
    assert.equal(prevented, false, 'contextmenu with selection must NOT call preventDefault()');
    const ctxCalls = calls.filter((c) => c.command === 'show_context_menu');
    assert.equal(ctxCalls.length, 0, 'show_context_menu must NOT be dispatched when text is selected');
  });

  test('VAL-WIN-003: contextMenuHandler drops primary click events (button === 0 && !ctrlKey) and allows right click or Ctrl+left click', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const contextMenuListeners = docListeners['contextmenu'] || [];

    function fireContextMenu(button, ctrlKey = false, timeStamp = undefined) {
      let prevented = false;
      const event = {
        button,
        ctrlKey,
        timeStamp,
        preventDefault: () => {
          prevented = true;
        }
      };
      for (const fn of contextMenuListeners) {
        fn(event);
      }
      return prevented;
    }

    // 1. Primary click (button 0 without ctrlKey): must be dropped
    assert.equal(fireContextMenu(0, false, 1000), false, 'button 0 without ctrlKey must not preventDefault');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 0, 'button 0 without ctrlKey must not invoke show_context_menu');

    // 2. Right click (button 2 without ctrlKey): must be handled
    assert.equal(fireContextMenu(2, false, 2000), true, 'button 2 must call preventDefault');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 1, 'button 2 must invoke show_context_menu');

    // 3. Ctrl + Left click (button 0 with ctrlKey): must be handled (macOS / standard gesture)
    assert.equal(fireContextMenu(0, true, 3000), true, 'Ctrl+button 0 must call preventDefault');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 2, 'Ctrl+button 0 must invoke show_context_menu');
  });

  test('VAL-WIN-003: contextMenuHandler enforces 300ms dismissal cooldown timestamp guard', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const contextMenuListeners = docListeners['contextmenu'] || [];

    function fireContextMenu(button = 2, ctrlKey = false, timeStamp = undefined) {
      let prevented = false;
      const event = {
        button,
        ctrlKey,
        timeStamp,
        preventDefault: () => {
          prevented = true;
        }
      };
      for (const fn of contextMenuListeners) {
        fn(event);
      }
      return prevented;
    }

    // First context menu at t=1000ms: allowed
    assert.equal(fireContextMenu(2, false, 1000), true, 'first contextmenu at t=1000 must be handled');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 1);

    // Repeated context menu at t=1150ms (< 300ms cooldown): dropped
    assert.equal(fireContextMenu(2, false, 1150), false, 'contextmenu at t=1150ms (< 300ms) must be dropped');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 1);

    // Another repeated context menu at t=1299ms (< 300ms from t=1000ms): dropped
    assert.equal(fireContextMenu(2, false, 1299), false, 'contextmenu at t=1299ms (< 300ms) must be dropped');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 1);

    // Context menu at t=1300ms (>= 300ms cooldown from t=1000ms): allowed
    assert.equal(fireContextMenu(2, false, 1300), true, 'contextmenu at t=1300ms (>= 300ms) must be handled');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 2);

    // Rapid event after t=1300 at t=1400 (< 300ms from t=1300): dropped
    assert.equal(fireContextMenu(2, false, 1400), false, 'contextmenu at t=1400ms must be dropped');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 2);

    // Event at t=1605 (>= 300ms from t=1300): allowed
    assert.equal(fireContextMenu(2, false, 1605), true, 'contextmenu at t=1605ms must be handled');
    assert.equal(calls.filter((c) => c.command === 'show_context_menu').length, 3);
  });

  test('test_web_notification_bridge shortcuts work when focus is in input/textarea without blocking typing', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const keydownListeners = docListeners['keydown'] || [];

    // Mock activeElement as an input element
    const inputEl = {
      tagName: 'INPUT',
      selectionStart: 5,
      selectionEnd: 5
    };
    harness.document.activeElement = inputEl;

    function fireKey(key, ctrlKey = false, shiftKey = false, extra = {}) {
      let prevented = false;
      const event = {
        ctrlKey,
        metaKey: false,
        shiftKey,
        altKey: false,
        key,
        target: inputEl,
        preventDefault: () => {
          prevented = true;
        },
        ...extra
      };
      for (const fn of keydownListeners) {
        fn(event);
      }
      return prevented;
    }

    // Standard typing should NOT be prevented
    assert.equal(fireKey('a', false, false), false, 'Normal typing "a" must not call preventDefault()');
    assert.equal(fireKey('1', false, false), false, 'Normal typing "1" must not call preventDefault()');
    assert.equal(fireKey('+', false, false), false, 'Normal typing "+" must not call preventDefault()');
    assert.equal(fireKey('-', false, false), false, 'Normal typing "-" must not call preventDefault()');
    assert.equal(fireKey('0', false, false), false, 'Normal typing "0" must not call preventDefault()');

    // Shortcuts should still trigger and call preventDefault
    // Ctrl+,
    assert.equal(fireKey(',', true, false), true, 'Ctrl+, in input must trigger');
    assert.ok(calls.some((c) => c.command === 'open_settings'), 'open_settings must be called');

    // F1
    assert.equal(fireKey('F1', false, false), true, 'F1 in input must trigger');
    assert.ok(calls.some((c) => c.command === 'open_help'), 'open_help must be called');

    // F5
    assert.equal(fireKey('F5', false, false), true, 'F5 in input must trigger');

    // Zoom In: Ctrl+=
    assert.equal(fireKey('=', true, false), true, 'Ctrl+= in input must trigger');

    // Zoom Out: Ctrl+-
    assert.equal(fireKey('-', true, false), true, 'Ctrl+- in input must trigger');

    // Reset Zoom: Ctrl+0
    assert.equal(fireKey('0', true, false), true, 'Ctrl+0 in input must trigger');
  });

  test('test_web_notification_bridge exposes openAbout on bridgeApi', () => {
    const harness = createContainerHarness();
    const { bridge, calls } = harness;
    assert.equal(typeof bridge.openAbout, 'function', 'openAbout must be exposed on bridgeApi');
    bridge.openAbout();
    const aboutCalls = calls.filter((c) => c.command === 'open_about');
    assert.equal(aboutCalls.length, 1, 'open_about IPC command must be dispatched');
  });
});

describe('VAL-GATEWAY-006: settings.html and settings.js gateway scopes & master toggle', () => {
  test('VAL-GATEWAY-006: settings.html exposes master connector switch and 4 scope checkboxes', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="connector-enabled"/, 'settings.html must expose master switch #connector-enabled');
    assert.match(html, /id="scope-files"/, 'settings.html must expose #scope-files');
    assert.match(html, /id="scope-exec"/, 'settings.html must expose #scope-exec');
    assert.match(html, /id="scope-browser"/, 'settings.html must expose #scope-browser');
    assert.match(html, /id="scope-desktop"/, 'settings.html must expose #scope-desktop');
  });

  test('VAL-GATEWAY-006: settings.js persists and loads connector scopes and master toggle', () => {
    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /connector_enabled/, 'settings.js must reference connector_enabled');
    assert.match(js, /scope_files/, 'settings.js must reference scope_files');
    assert.match(js, /scope_exec/, 'settings.js must reference scope_exec');
    assert.match(js, /scope_browser/, 'settings.js must reference scope_browser');
    assert.match(js, /scope_desktop/, 'settings.js must reference scope_desktop');
  });

  test('VAL-A0-007: settings.html and settings.js support configurable workspace_path', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="workspace-path"/, 'settings.html must expose #workspace-path');
    assert.match(html, /name="workspace_path"/, 'settings.html must set name="workspace_path"');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /workspace_path/, 'settings.js must reference workspace_path');
    assert.match(js, /workspacePathInput/, 'settings.js must bind workspacePathInput');
  });

  test('VAL-HOTKEY-001: settings.html exposes shortcuts section, #global-hotkey and #reset-hotkey-btn', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="shortcuts-section"/, 'settings.html must expose #shortcuts-section');
    assert.match(html, /id="global-hotkey"/, 'settings.html must expose #global-hotkey');
    assert.match(html, /id="reset-hotkey-btn"/, 'settings.html must expose #reset-hotkey-btn');
    assert.match(html, /href="help\.html"/, 'settings.html must link to help.html');
  });

  test('VAL-HOTKEY-002: settings.js handles global_hotkey loading, saving, and reset', () => {
    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /global_hotkey/, 'settings.js must reference global_hotkey');
    assert.match(js, /reset-hotkey-btn/, 'settings.js must bind reset-hotkey-btn');
  });

  test('VAL-HOTKEY-003: container.js dispatches open_help on F1 keydown', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    let prevented = false;
    const event = {
      ctrlKey: false,
      metaKey: false,
      key: 'F1',
      preventDefault: () => {
        prevented = true;
      }
    };
    const keydownListeners = docListeners['keydown'] || [];
    assert.ok(keydownListeners.length > 0, 'keydown listener must be registered');
    for (const fn of keydownListeners) {
      fn(event);
    }
    assert.equal(prevented, true, 'F1 must call preventDefault()');
    const openHelpCalls = calls.filter((c) => c.command === 'open_help');
    assert.equal(openHelpCalls.length, 1, 'open_help IPC command must be dispatched');
  });

  test('container bridge toggleFocusMode toggles aqvn-focus-mode on body', () => {
    const harness = createContainerHarness();
    const { bridge, document } = harness;
    assert.ok(typeof bridge.toggleFocusMode === 'function', 'toggleFocusMode must be exposed');

    const first = bridge.toggleFocusMode();
    assert.equal(first, true, 'first toggle turns focus mode on');
    assert.ok(document.body.classList.contains('aqvn-focus-mode'), 'body must have aqvn-focus-mode');

    const second = bridge.toggleFocusMode();
    assert.equal(second, false, 'second toggle turns focus mode off');
    assert.equal(document.body.classList.contains('aqvn-focus-mode'), false, 'aqvn-focus-mode removed');
  });

  test('container bridge toggles focus mode on F11 and Ctrl+Shift+F', () => {
    const harness = createContainerHarness();
    const { document, docListeners } = harness;
    const keydownListeners = docListeners['keydown'] || [];

    let f11Prevented = false;
    const f11Event = {
      ctrlKey: false,
      shiftKey: false,
      metaKey: false,
      key: 'F11',
      preventDefault: () => {
        f11Prevented = true;
      }
    };
    for (const fn of keydownListeners) {
      fn(f11Event);
    }
    assert.equal(f11Prevented, true, 'F11 calls preventDefault');
    assert.ok(document.body.classList.contains('aqvn-focus-mode'), 'F11 turns on aqvn-focus-mode');

    let csfPrevented = false;
    const csfEvent = {
      ctrlKey: true,
      shiftKey: true,
      metaKey: false,
      key: 'F',
      preventDefault: () => {
        csfPrevented = true;
      }
    };
    for (const fn of keydownListeners) {
      fn(csfEvent);
    }
    assert.equal(csfPrevented, true, 'Ctrl+Shift+F calls preventDefault');
    assert.equal(document.body.classList.contains('aqvn-focus-mode'), false, 'Ctrl+Shift+F turns off aqvn-focus-mode');
  });

  test('VAL-WIN-007: container bridge dispatches toggle_fullscreen IPC on F11', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const keydownListeners = docListeners['keydown'] || [];
    let prevented = false;
    const event = {
      ctrlKey: false,
      metaKey: false,
      key: 'F11',
      preventDefault: () => {
        prevented = true;
      }
    };
    for (const fn of keydownListeners) {
      fn(event);
    }
    assert.equal(prevented, true, 'F11 calls preventDefault');
    const toggleCalls = calls.filter((c) => c.command === 'toggle_fullscreen');
    assert.equal(toggleCalls.length, 1, 'toggle_fullscreen IPC command must be dispatched on F11');
  });

  test('settings.html and settings.js support language switcher and close to tray', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="app-language"/, 'settings.html must expose #app-language select');
    assert.match(html, /id="close-to-tray"/, 'settings.html must expose #close-to-tray checkbox');
    assert.match(html, /id="lang-toggle-btn"/, 'settings.html must expose #lang-toggle-btn');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /app_language/, 'settings.js must reference app_language');
    assert.match(js, /close_to_tray/, 'settings.js must reference close_to_tray');
    assert.match(js, /SETTINGS_I18N/, 'settings.js must define SETTINGS_I18N');
  });

  test('VAL-MENU-005: settings.html and settings.js support show_menu_bar toggle in General section', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="show-menu-bar"/, 'settings.html must expose #show-menu-bar');
    assert.match(html, /name="show_menu_bar"/, 'settings.html must set name="show_menu_bar"');
    assert.match(html, /data-i18n="show_menu_bar_label"/, 'settings.html must have data-i18n="show_menu_bar_label"');
    assert.match(html, /data-i18n="show_menu_bar_help"/, 'settings.html must have data-i18n="show_menu_bar_help"');

    // Verify it is inside #general-section
    const generalMatch = html.match(/<section[^>]*id="general-section"[^>]*>([\s\S]*?)<\/section>/);
    assert.ok(generalMatch, 'settings.html must have #general-section');
    assert.match(generalMatch[1], /id="show-menu-bar"/, '#general-section must contain #show-menu-bar');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /show_menu_bar/, 'settings.js must reference show_menu_bar');
    assert.match(js, /showMenuBarInput/, 'settings.js must bind showMenuBarInput');
    assert.match(js, /show_menu_bar_label/, 'settings.js must define show_menu_bar_label in i18n');
    assert.match(js, /show_menu_bar_help/, 'settings.js must define show_menu_bar_help in i18n');
  });

  test('VAL-UX-002: settings.html and settings.js support task_chime_enabled toggle in General section', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="task-chime-enabled"/, 'settings.html must expose #task-chime-enabled');
    assert.match(html, /name="task_chime_enabled"/, 'settings.html must set name="task_chime_enabled"');
    assert.match(html, /data-i18n="task_chime_enabled_label"/, 'settings.html must have data-i18n="task_chime_enabled_label"');
    assert.match(html, /data-i18n="task_chime_enabled_help"/, 'settings.html must have data-i18n="task_chime_enabled_help"');

    const generalMatch = html.match(/<section[^>]*id="general-section"[^>]*>([\s\S]*?)<\/section>/);
    assert.ok(generalMatch, 'settings.html must have #general-section');
    assert.match(generalMatch[1], /id="task-chime-enabled"/, '#general-section must contain #task-chime-enabled');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /task_chime_enabled/, 'settings.js must reference task_chime_enabled');
    assert.match(js, /taskChimeEnabledInput/, 'settings.js must bind taskChimeEnabledInput');
    assert.match(js, /task_chime_enabled_label/, 'settings.js must define task_chime_enabled_label in i18n');
    assert.match(js, /task_chime_enabled_help/, 'settings.js must define task_chime_enabled_help in i18n');
  });

  test('VAL-UX-002: settings.html and settings.js support task_chime_volume slider and task_chime_tone selector with live preview', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="task-chime-volume"/, 'settings.html must expose #task-chime-volume');
    assert.match(html, /name="task_chime_volume"/, 'settings.html must set name="task_chime_volume"');
    assert.match(html, /id="task-chime-tone"/, 'settings.html must expose #task-chime-tone');
    assert.match(html, /name="task_chime_tone"/, 'settings.html must set name="task_chime_tone"');
    assert.match(html, /id="preview-chime-btn"/, 'settings.html must expose #preview-chime-btn');

    // Tone options check
    assert.match(html, /value="classic"/, 'tone options must include classic');
    assert.match(html, /value="bell"/, 'tone options must include bell');
    assert.match(html, /value="beep"/, 'tone options must include beep');
    assert.match(html, /value="subtle"/, 'tone options must include subtle');

    const generalMatch = html.match(/<section[^>]*id="general-section"[^>]*>([\s\S]*?)<\/section>/);
    assert.ok(generalMatch, 'settings.html must have #general-section');
    assert.match(generalMatch[1], /id="task-chime-volume"/, '#general-section must contain #task-chime-volume');
    assert.match(generalMatch[1], /id="task-chime-tone"/, '#general-section must contain #task-chime-tone');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /task_chime_volume/, 'settings.js must reference task_chime_volume');
    assert.match(js, /task_chime_tone/, 'settings.js must reference task_chime_tone');
    assert.match(js, /taskChimeVolumeInput/, 'settings.js must bind taskChimeVolumeInput');
    assert.match(js, /taskChimeToneInput/, 'settings.js must bind taskChimeToneInput');
    assert.match(js, /previewChimeBtn/, 'settings.js must bind previewChimeBtn');
    assert.match(js, /task_chime_volume_label/, 'settings.js must define task_chime_volume_label in i18n');
    assert.match(js, /task_chime_tone_label/, 'settings.js must define task_chime_tone_label in i18n');
    assert.match(js, /preview_chime_btn/, 'settings.js must define preview_chime_btn in i18n');
  });

  test('VAL-CACHE-004: settings.html and settings.js support clear_disk_cache command in Settings', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="clear-disk-cache-btn"/, 'settings.html must expose #clear-disk-cache-btn');
    assert.match(html, /data-i18n="clear_disk_cache_btn"/, 'settings.html must have data-i18n="clear_disk_cache_btn"');

    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /clear_disk_cache/, 'settings.js must reference clear_disk_cache');
    assert.match(js, /clearDiskCacheBtn/, 'settings.js must bind clearDiskCacheBtn');
    assert.match(js, /clear_disk_cache_btn/, 'settings.js must define clear_disk_cache_btn in i18n');
  });

  test('help.html supports close_current_window and language switcher', () => {
    const htmlPath = new URL('../../ui/help.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /close_current_window/, 'help.html must invoke close_current_window');
    assert.match(html, /id="lang-toggle-btn"/, 'help.html must expose #lang-toggle-btn');
    assert.match(html, /HELP_I18N/, 'help.html must define HELP_I18N');
    assert.match(html, /closeHelpWindow/, 'help.html must implement closeHelpWindow()');
  });

  test('about.html displays version tag', () => {
    const htmlPath = new URL('../../ui/about.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');
    assert.match(html, /<span class="version-tag">v0\.1\.[89]<\/span>/, 'about.html must show current version');
  });
});

describe('VAL-HOTKEY-003 & VAL-HOTKEY-004: Container bridge shortcuts and IPC error visibility', () => {
  test('VAL-HOTKEY-004: invokeIpc logs warning when IPC fails', async () => {
    const warnings = [];
    const harness = createContainerHarness({
      ipcReject: true,
      onWarn: (msg, ...args) => warnings.push([msg, ...args])
    });
    const { bridge } = harness;
    assert.equal(typeof bridge.invokeIpc, 'function', 'invokeIpc must be exposed on bridge');
    await bridge.invokeIpc('test_command', { foo: 'bar' });
    assert.ok(warnings.length > 0, 'console.warn must be called when IPC fails');
    assert.match(String(warnings[0][0]), /IPC error/);
  });

  test('VAL-HOTKEY-003: installKeyboardShortcuts handles Equal, Minus, Digit0, NumpadAdd, NumpadSubtract, Numpad0, F1, F5, Comma', () => {
    const harness = createContainerHarness();
    const { calls, docListeners } = harness;
    const keydownListeners = docListeners['keydown'] || [];

    let simulatedTime = 2000;
    function fireKey(key, ctrlKey, shiftKey, code) {
      let prevented = false;
      const event = {
        key,
        code,
        ctrlKey,
        metaKey: false,
        shiftKey,
        altKey: false,
        timeStamp: simulatedTime,
        preventDefault: () => {
          prevented = true;
        }
      };
      simulatedTime += 200;
      for (const fn of keydownListeners) {
        fn(event);
      }
      return prevented;
    }

    // F1 -> open_help
    assert.equal(fireKey('F1', false, false, 'F1'), true, 'F1 must be handled');
    assert.ok(calls.some((c) => c.command === 'open_help'), 'F1 invokes open_help');

    // F5 -> hardRefresh
    assert.equal(fireKey('F5', false, false, 'F5'), true, 'F5 must be handled');
    assert.equal(harness.getReloaded(), true, 'F5 triggers reload');

    // Comma -> open_settings
    assert.equal(fireKey(',', true, false, 'Comma'), true, 'Ctrl+Comma must be handled');
    assert.ok(calls.some((c) => c.command === 'open_settings'), 'Ctrl+Comma invokes open_settings');

    // Equal -> Zoom In
    const zoomBeforeEqual = parseFloat(harness.document.documentElement.style.zoom || '1.0');
    assert.equal(fireKey('=', true, false, 'Equal'), true, 'Ctrl+Equal must be handled');
    assert.ok(parseFloat(harness.document.documentElement.style.zoom) > zoomBeforeEqual, 'Ctrl+Equal zooms in');

    // Minus -> Zoom Out
    const zoomBeforeMinus = parseFloat(harness.document.documentElement.style.zoom);
    assert.equal(fireKey('-', true, false, 'Minus'), true, 'Ctrl+Minus must be handled');
    assert.ok(parseFloat(harness.document.documentElement.style.zoom) < zoomBeforeMinus, 'Ctrl+Minus zooms out');

    // Digit0 -> Reset Zoom
    assert.equal(fireKey('0', true, false, 'Digit0'), true, 'Ctrl+Digit0 must be handled');
    assert.equal(harness.document.documentElement.style.zoom, '1', 'Ctrl+Digit0 resets zoom to 1');

    // NumpadAdd -> Zoom In
    assert.equal(fireKey('+', true, false, 'NumpadAdd'), true, 'Ctrl+NumpadAdd must be handled');
    assert.equal(harness.document.documentElement.style.zoom, '1.1', 'Ctrl+NumpadAdd zooms in');

    // NumpadSubtract -> Zoom Out
    assert.equal(fireKey('-', true, false, 'NumpadSubtract'), true, 'Ctrl+NumpadSubtract must be handled');
    assert.equal(harness.document.documentElement.style.zoom, '1', 'Ctrl+NumpadSubtract zooms out');

    // Numpad0 -> Reset Zoom
    fireKey('+', true, false, 'NumpadAdd');
    assert.equal(harness.document.documentElement.style.zoom, '1.1');
    assert.equal(fireKey('0', true, false, 'Numpad0'), true, 'Ctrl+Numpad0 must be handled');
    assert.equal(harness.document.documentElement.style.zoom, '1', 'Ctrl+Numpad0 resets zoom to 1');
  });

  test('VAL-HOTKEY-003: cross-iframe key event relay via postMessage and nested iframe key interception', () => {
    const harness = createContainerHarness();
    const { docListeners, bridge } = harness;

    // Relay via window message
    const messageListeners = docListeners['message'] || [];
    assert.ok(messageListeners.length > 0, 'window message listener must be registered for iframe relay');

    // Simulate cross-iframe message event
    for (const fn of messageListeners) {
      fn({
        data: {
          type: 'aqvn:keydown',
          key: '=',
          code: 'Equal',
          ctrlKey: true,
          metaKey: false,
          shiftKey: false,
          altKey: false,
          timeStamp: 3000
        }
      });
    }
    assert.equal(harness.document.documentElement.style.zoom, '1.1', 'relayed Ctrl+= must zoom in');

    // Nested iframe element listener
    const iframeListeners = {};
    const fakeIframe = {
      tagName: 'IFRAME',
      contentWindow: {
        document: {
          addEventListener: (type, fn) => {
            (iframeListeners[type] = iframeListeners[type] || []).push(fn);
          }
        }
      },
      addEventListener: () => {}
    };

    assert.equal(typeof bridge.hookIframe, 'function', 'hookIframe must be exposed');
    bridge.hookIframe(fakeIframe);
    assert.ok((iframeListeners['keydown'] || []).length > 0, 'iframe keydown listener installed');

    let prevented = false;
    for (const fn of iframeListeners['keydown']) {
      fn({
        key: 'F1',
        code: 'F1',
        ctrlKey: false,
        metaKey: false,
        altKey: false,
        preventDefault: () => {
          prevented = true;
        }
      });
    }
    assert.equal(prevented, true, 'iframe F1 keydown calls preventDefault');
    assert.ok(harness.calls.some((c) => c.command === 'open_help'), 'iframe F1 relayed to open_help');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-UI-004: Connection Diagnostics Modal & Network Recovery Status */
/* ------------------------------------------------------------------ */

describe('VAL-UI-004: Connection Diagnostics Modal and Network Recovery Status', () => {
  test('VAL-UI-004: index.html defines interactive connection status badge and diagnostics modal', () => {
    const html = readIndexHtml();

    assert.match(
      html,
      /id="connection-status-badge"/,
      'ui/index.html must expose interactive #connection-status-badge'
    );
    assert.match(
      html,
      /id="diagnostics-modal"/,
      'ui/index.html must expose #diagnostics-modal dialog'
    );
    assert.match(
      html,
      /id="diagnostics-title"/,
      'ui/index.html must expose #diagnostics-title'
    );
    assert.match(
      html,
      /id="diagnostics-latency"|id="diag-latency"/,
      'ui/index.html must expose latency metric element'
    );
    assert.match(
      html,
      /id="diagnostics-proxy-port"|id="diag-proxy-port"/,
      'ui/index.html must expose proxy port metric element'
    );
    assert.match(
      html,
      /id="diagnostics-active-profile"|id="diagnostics-server-profile"/,
      'ui/index.html must expose active/server profile metric element'
    );
    assert.match(
      html,
      /id="reconnect-now-btn"|id="reconnect-btn"/,
      'ui/index.html must expose manual reconnect button'
    );
    assert.match(
      html,
      /Reconnect Now/,
      'reconnect button in index.html must label Reconnect Now action'
    );
    assert.match(
      html,
      /id="close-diagnostics-btn"/,
      'ui/index.html must expose #close-diagnostics-btn'
    );
  });

  test('VAL-UI-004: ui/container.js exposes connection status tracking across Connected, Reconnecting, and Offline', () => {
    const harness = createContainerHarness();
    const { bridge } = harness;

    assert.equal(
      typeof bridge.setConnectionStatus,
      'function',
      'bridge.setConnectionStatus must be function'
    );
    assert.equal(
      typeof bridge.getConnectionStatus,
      'function',
      'bridge.getConnectionStatus must be function'
    );

    // Initial default or connected
    const initial = bridge.getConnectionStatus();
    assert.ok(
      ['connected', 'reconnecting', 'offline'].includes(initial),
      'initial status must be connected, reconnecting, or offline'
    );

    // Set connected
    const resConnected = bridge.setConnectionStatus('connected');
    assert.equal(resConnected, 'connected');
    assert.equal(bridge.getConnectionStatus(), 'connected');

    // Set reconnecting
    const resReconnecting = bridge.setConnectionStatus('reconnecting');
    assert.equal(resReconnecting, 'reconnecting');
    assert.equal(bridge.getConnectionStatus(), 'reconnecting');

    // Set offline
    const resOffline = bridge.setConnectionStatus('offline');
    assert.equal(resOffline, 'offline');
    assert.equal(bridge.getConnectionStatus(), 'offline');
  });

  test('VAL-UI-004: clicking status badge opens Diagnostics modal displaying metrics', () => {
    const harness = createContainerHarness({
      localStorage: {
        aqvn_active_profile: 'https://prod.workspace.aqvn'
      }
    });
    const { bridge, elements, document, createElement } = harness;

    // Set up proxy configuration
    harness.context.window.__AQVN_PROXY__ = { port: 38451, url: 'http://[IP]:38451' };

    // Register DOM mocks
    const badge = createElement('connection-status-badge');
    const modal = createElement('diagnostics-modal');
    modal.hidden = true;
    const title = createElement('diagnostics-title');
    const latencyEl = createElement('diagnostics-latency');
    const portEl = createElement('diagnostics-proxy-port');
    const profileEl = createElement('diagnostics-active-profile');
    const statusEl = createElement('diagnostics-connection-status');
    const closeBtn = createElement('close-diagnostics-btn');
    const reconnectBtn = createElement('reconnect-now-btn');

    elements['connection-status-badge'] = badge;
    elements['diagnostics-modal'] = modal;
    elements['diagnostics-title'] = title;
    elements['diagnostics-latency'] = latencyEl;
    elements['diagnostics-proxy-port'] = portEl;
    elements['diagnostics-active-profile'] = profileEl;
    elements['diagnostics-connection-status'] = statusEl;
    elements['close-diagnostics-btn'] = closeBtn;
    elements['reconnect-now-btn'] = reconnectBtn;

    bridge.installDiagnostics(document);

    // Clicking status badge opens modal
    badge.dispatch('click', { preventDefault: () => {} });

    assert.equal(modal.hidden, false, 'modal.hidden must be false after badge click');
    assert.equal(bridge.isDiagnosticsModalOpen(), true, 'isDiagnosticsModalOpen must return true');

    // Check displayed metrics
    assert.equal(portEl.textContent, '38451', 'proxy port metric must show 38451');
    assert.equal(
      profileEl.textContent,
      'https://prod.workspace.aqvn',
      'active profile metric must show configured profile'
    );
    assert.ok(latencyEl.textContent.length > 0, 'latency metric must be populated');

    // Clicking close button closes modal
    closeBtn.dispatch('click', { preventDefault: () => {} });
    assert.equal(modal.hidden, true, 'modal.hidden must be true after close click');
    assert.equal(bridge.isDiagnosticsModalOpen(), false, 'isDiagnosticsModalOpen must return false');
  });

  test('VAL-UI-004: Escape key closes Diagnostics modal when open', () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, docListeners, createElement } = harness;

    const modal = createElement('diagnostics-modal');
    modal.hidden = true;
    elements['diagnostics-modal'] = modal;
    elements['close-diagnostics-btn'] = createElement('close-diagnostics-btn');

    bridge.installDiagnostics(document);
    bridge.openDiagnosticsModal();
    assert.equal(bridge.isDiagnosticsModalOpen(), true);

    const keydownListeners = docListeners['keydown'] || [];
    let prevented = false;
    for (const fn of keydownListeners) {
      fn({
        key: 'Escape',
        preventDefault: () => {
          prevented = true;
        }
      });
    }

    assert.equal(bridge.isDiagnosticsModalOpen(), false, 'Escape key must close diagnostics modal');
  });

  test('VAL-UI-004: clicking Reconnect Now triggers manual reconnect action and sets reconnecting state', async () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, calls, createElement } = harness;

    const modal = createElement('diagnostics-modal');
    modal.hidden = false;
    elements['diagnostics-modal'] = modal;
    const badge = createElement('connection-status-badge');
    elements['connection-status-badge'] = badge;
    const reconnectBtn = createElement('reconnect-now-btn');
    elements['reconnect-now-btn'] = reconnectBtn;

    let reconnectEventDispatched = false;
    harness.context.window.addEventListener('aqvn:reconnect', () => {
      reconnectEventDispatched = true;
    });

    bridge.installDiagnostics(document);

    // Initial state: connected
    bridge.setConnectionStatus('connected');

    // Trigger reconnect
    const promise = bridge.reconnectNow();
    assert.equal(
      bridge.getConnectionStatus(),
      'reconnecting',
      'status must transition to reconnecting immediately on reconnectNow'
    );
    assert.equal(badge.dataset.state, 'reconnecting', 'badge state must be reconnecting');
    assert.ok(reconnectEventDispatched, 'aqvn:reconnect window event must be dispatched');

    await promise;

    const reconnectCalls = calls.filter((c) => c.command === 'reconnect_network');
    assert.equal(reconnectCalls.length, 1, 'reconnect_network IPC must be dispatched');
  });
});

describe('VAL-RES-007: Settings save button handler deduplication', () => {
  test('VAL-RES-007: settings.js deduplicates save button handler and registers submit listener on form', () => {
    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.doesNotMatch(
      js,
      /saveBtn\.addEventListener\(\s*['"]click['"]\s*,\s*saveSettings\s*\)/,
      'settings.js must not register duplicate click listener on saveBtn'
    );
    assert.match(
      js,
      /settingsForm\.addEventListener\(\s*['"]submit['"]\s*,\s*saveSettings\s*\)/,
      'settings.js must register submit listener on settingsForm'
    );
  });

  test('VAL-RES-007: tools/cross_area_e2e.sh defaults to TMPDIR without hardcoded user paths', () => {
    const shPath = new URL('../cross_area_e2e.sh', import.meta.url);
    const sh = fs.readFileSync(shPath, 'utf8');

    assert.doesNotMatch(
      sh,
      /\/home\/babim/,
      'cross_area_e2e.sh must not contain hardcoded user path /home/babim'
    );
    assert.doesNotMatch(
      sh,
      /\/home\/[a-zA-Z0-9_-]+/,
      'cross_area_e2e.sh must not contain hardcoded /home/<user> paths'
    );
    assert.match(
      sh,
      /TMPDIR/,
      'cross_area_e2e.sh must default temporary / mission directories using TMPDIR'
    );
  });
});

describe('VAL-THEME-001 & VAL-THEME-002: 4-Theme engine and persistence', () => {
  test('VAL-THEME-001: ui/chat.css defines 4 themes with distinct OKLCH palettes', () => {
    const cssPath = new URL('../../ui/chat.css', import.meta.url);
    const css = fs.readFileSync(cssPath, 'utf8');

    assert.match(css, /:root|\[data-theme=["']aqvn-dark["']\]/, 'CSS must support aqvn-dark');
    assert.match(css, /\[data-theme=["']oled-black["']\]/, 'CSS must support oled-black');
    assert.match(css, /\[data-theme=["']cyberpunk-terminal["']\]/, 'CSS must support cyberpunk-terminal');
    assert.match(css, /\[data-theme=["']clean-studio-light["']\]/, 'CSS must support clean-studio-light');
    assert.match(css, /oklch\(/, 'CSS must use OKLCH color palettes');
    assert.match(css, /\[data-theme=["']clean-studio-light["']\][^{]*\{[^}]*color-scheme:\s*light/i, 'clean-studio-light must declare light color-scheme');
  });

  test('VAL-THEME-001: ui/index.html exposes topbar theme toggle button', () => {
    const htmlPath = new URL('../../ui/index.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="theme-toggle-btn"/, 'ui/index.html must expose #theme-toggle-btn in topbar');
  });

  test('VAL-THEME-001: clicking topbar theme toggle cycles through all 4 themes', () => {
    const harness = createHarness();
    harness.exports.init();
    const { exports, document } = harness;
    const dom = exports.dom;

    assert.ok(dom.themeToggleBtn, 'dom.themeToggleBtn must exist');
    const initialTheme = document.documentElement.getAttribute('data-theme') || 'aqvn-dark';
    assert.equal(initialTheme, 'aqvn-dark', 'Initial theme must be aqvn-dark');

    // 1st click -> oled-black
    dom.themeToggleBtn.dispatch('click', {});
    assert.equal(document.documentElement.getAttribute('data-theme'), 'oled-black');
    assert.equal(harness.store['aqvn_theme'], 'oled-black');

    // 2nd click -> cyberpunk-terminal
    dom.themeToggleBtn.dispatch('click', {});
    assert.equal(document.documentElement.getAttribute('data-theme'), 'cyberpunk-terminal');
    assert.equal(harness.store['aqvn_theme'], 'cyberpunk-terminal');

    // 3rd click -> clean-studio-light
    dom.themeToggleBtn.dispatch('click', {});
    assert.equal(document.documentElement.getAttribute('data-theme'), 'clean-studio-light');
    assert.equal(harness.store['aqvn_theme'], 'clean-studio-light');

    // 4th click -> wraps back to aqvn-dark
    dom.themeToggleBtn.dispatch('click', {});
    assert.equal(document.documentElement.getAttribute('data-theme'), 'aqvn-dark');
    assert.equal(harness.store['aqvn_theme'], 'aqvn-dark');
  });

  test('VAL-THEME-001: ui/settings.html exposes theme dropdown in General section', () => {
    const htmlPath = new URL('../../ui/settings.html', import.meta.url);
    const html = fs.readFileSync(htmlPath, 'utf8');

    assert.match(html, /id="app-theme"/, 'ui/settings.html must expose #app-theme');
    assert.match(html, /value="aqvn-dark"/, 'ui/settings.html options must include aqvn-dark');
    assert.match(html, /value="oled-black"/, 'ui/settings.html options must include oled-black');
    assert.match(html, /value="cyberpunk-terminal"/, 'ui/settings.html options must include cyberpunk-terminal');
    assert.match(html, /value="clean-studio-light"/, 'ui/settings.html options must include clean-studio-light');
  });

  test('VAL-THEME-002: theme choice persists in settings.json and localStorage across reloads', async () => {
    const jsPath = new URL('../../ui/settings.js', import.meta.url);
    const js = fs.readFileSync(jsPath, 'utf8');

    assert.match(js, /app_theme/, 'settings.js must handle app_theme setting');
    assert.match(js, /aqvn_theme/, 'settings.js must persist aqvn_theme to localStorage');

    const harness = createHarness();
    harness.store['aqvn_theme'] = 'cyberpunk-terminal';
    harness.exports.init();
    assert.equal(
      harness.document.documentElement.getAttribute('data-theme'),
      'cyberpunk-terminal',
      'chat.js must restore persisted theme on init'
    );
  });

  test('VAL-THEME-002: index.html and settings.html restore theme in head to eliminate flicker', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    const settingsHtml = fs.readFileSync(new URL('../../ui/settings.html', import.meta.url), 'utf8');

    const headIndex = indexHtml.slice(0, indexHtml.indexOf('</head>'));
    const headSettings = settingsHtml.slice(0, settingsHtml.indexOf('</head>'));

    assert.match(headIndex, /aqvn_theme.*data-theme/s, 'index.html head must apply theme synchronously');
    assert.match(headSettings, /aqvn_theme.*data-theme/s, 'settings.html head must apply theme synchronously');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-SEARCH-001: In-chat search drawer toggle and navigation        */
/* ------------------------------------------------------------------ */

describe('VAL-SEARCH-001: In-chat search drawer toggle and navigation', () => {
  test('VAL-SEARCH-001: ui/index.html exposes search toggle button, drawer, input, count, and nav buttons', () => {
    const html = readIndexHtml();

    assert.match(html, /id="search-toggle-btn"/, 'ui/index.html must declare #search-toggle-btn');
    assert.match(html, /id="search-drawer"/, 'ui/index.html must declare #search-drawer');
    assert.match(html, /id="search-input"/, 'ui/index.html must declare #search-input');
    assert.match(html, /id="search-count"/, 'ui/index.html must declare #search-count');
    assert.match(html, /id="search-prev-btn"/, 'ui/index.html must declare #search-prev-btn');
    assert.match(html, /id="search-next-btn"/, 'ui/index.html must declare #search-next-btn');
    assert.match(html, /id="search-close-btn"/, 'ui/index.html must declare #search-close-btn');
  });

  test('VAL-SEARCH-001: Ctrl+F and search toggle button open #search-drawer and focus input', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state } = harness.exports;

    assert.ok(dom.searchDrawer, 'dom.searchDrawer must exist');
    assert.equal(dom.searchDrawer.hidden, true, 'search drawer starts hidden');

    // Ctrl+F opens search drawer
    let prevented = false;
    harness.document.fire('keydown', {
      ctrlKey: true,
      key: 'f',
      preventDefault: () => {
        prevented = true;
      }
    });

    assert.equal(prevented, true, 'Ctrl+F must call preventDefault()');
    assert.equal(dom.searchDrawer.hidden, false, 'Ctrl+F must open #search-drawer');
    assert.equal(state.searchOpen, true, 'state.searchOpen must be true');

    // Escape closes search drawer
    harness.document.fire('keydown', {
      key: 'Escape',
      preventDefault: () => {}
    });
    assert.equal(dom.searchDrawer.hidden, true, 'Escape must close #search-drawer');
    assert.equal(state.searchOpen, false, 'state.searchOpen must be false');

    // Clicking search toggle button toggles drawer
    dom.searchToggleBtn.dispatch('click', {});
    assert.equal(dom.searchDrawer.hidden, false, 'Clicking #search-toggle-btn must open drawer');
    assert.equal(state.searchOpen, true);

    // Clicking close button closes drawer
    dom.searchCloseBtn.dispatch('click', {});
    assert.equal(dom.searchDrawer.hidden, true, 'Clicking #search-close-btn must close drawer');
    assert.equal(state.searchOpen, false);
  });

  test('VAL-SEARCH-001: Typing highlights matching text across messages and shows match count', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, upsertLog } = harness.exports;

    // Seed conversation messages
    upsertLog({ id: 'msg-1', type: 'user', content: 'Need to run cargo test suite' });
    upsertLog({ id: 'msg-2', type: 'code_exe', content: 'cargo test --manifest-path src-tauri/Cargo.toml' });
    upsertLog({ id: 'msg-3', type: 'agent', content: 'All tests finished cleanly' });

    // Open search drawer
    dom.searchToggleBtn.dispatch('click', {});

    // Type "test"
    dom.searchInput.value = 'test';
    dom.searchInput.dispatch('input', {});

    // Match count: "test" appears in msg-1 (1), msg-2 (1), msg-3 (1) -> 3 matches
    assert.equal(state.searchMatchesCount, 3, 'Must detect 3 matches across messages');
    assert.match(dom.searchCount.textContent, /1 of 3|1 \/ 3/, 'Search count badge must show 1 of 3');

    const row1 = state.rows.get('msg-1');
    const row2 = state.rows.get('msg-2');
    const row3 = state.rows.get('msg-3');

    assert.match(
      row1.lastChild.innerHTML,
      /<mark class="search-match active">test<\/mark>/,
      'Active match must have active class'
    );
    assert.match(
      row2.lastChild.innerHTML,
      /<mark class="search-match">test<\/mark>/,
      'Subsequent matches must have search-match class'
    );
    assert.match(
      row3.lastChild.innerHTML,
      /<mark class="search-match">test<\/mark>/,
      'Subsequent matches must have search-match class'
    );
  });

  test('VAL-SEARCH-001: Enter and Shift+Enter in search input navigate through matches', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, upsertLog } = harness.exports;

    upsertLog({ id: 'm-1', type: 'user', content: 'alpha keyword first' });
    upsertLog({ id: 'm-2', type: 'code_exe', content: 'alpha keyword second' });
    upsertLog({ id: 'm-3', type: 'agent', content: 'alpha keyword third' });

    dom.searchToggleBtn.dispatch('click', {});
    dom.searchInput.value = 'keyword';
    dom.searchInput.dispatch('input', {});

    assert.equal(state.searchMatchesCount, 3);
    assert.match(dom.searchCount.textContent, /1 of 3|1 \/ 3/);

    // Enter -> advances to match 2
    dom.searchInput.dispatch('keydown', { key: 'Enter', shiftKey: false, preventDefault: () => {} });
    assert.match(dom.searchCount.textContent, /2 of 3|2 \/ 3/, 'Enter must advance to match 2');
    assert.match(
      state.rows.get('m-2').lastChild.innerHTML,
      /<mark class="search-match active">keyword<\/mark>/,
      'Match 2 must now be active'
    );

    // Enter -> advances to match 3
    dom.searchInput.dispatch('keydown', { key: 'Enter', shiftKey: false, preventDefault: () => {} });
    assert.match(dom.searchCount.textContent, /3 of 3|3 \/ 3/, 'Enter must advance to match 3');
    assert.match(
      state.rows.get('m-3').lastChild.innerHTML,
      /<mark class="search-match active">keyword<\/mark>/,
      'Match 3 must now be active'
    );

    // Enter -> wraps around to match 1
    dom.searchInput.dispatch('keydown', { key: 'Enter', shiftKey: false, preventDefault: () => {} });
    assert.match(dom.searchCount.textContent, /1 of 3|1 \/ 3/, 'Enter must wrap to match 1');
    assert.match(
      state.rows.get('m-1').lastChild.innerHTML,
      /<mark class="search-match active">keyword<\/mark>/,
      'Match 1 must now be active again'
    );

    // Shift+Enter -> wraps backward to match 3
    dom.searchInput.dispatch('keydown', { key: 'Enter', shiftKey: true, preventDefault: () => {} });
    assert.match(dom.searchCount.textContent, /3 of 3|3 \/ 3/, 'Shift+Enter must navigate backward to match 3');
    assert.match(
      state.rows.get('m-3').lastChild.innerHTML,
      /<mark class="search-match active">keyword<\/mark>/,
      'Match 3 must be active on reverse navigation'
    );

    // Nav buttons #search-prev-btn and #search-next-btn
    dom.searchPrevBtn.dispatch('click', {});
    assert.match(dom.searchCount.textContent, /2 of 3|2 \/ 3/, 'searchPrevBtn must move to match 2');

    dom.searchNextBtn.dispatch('click', {});
    assert.match(dom.searchCount.textContent, /3 of 3|3 \/ 3/, 'searchNextBtn must move to match 3');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-SEARCH-002: Search category filter pills                       */
/* ------------------------------------------------------------------ */

describe('VAL-SEARCH-002: Search category filter pills', () => {
  test('VAL-SEARCH-002: ui/index.html exposes category filter pills', () => {
    const html = readIndexHtml();

    assert.match(html, /class="[^"]*filter-pill[^"]*"[^>]*data-category="all"/, 'index.html must have All filter pill');
    assert.match(html, /class="[^"]*filter-pill[^"]*"[^>]*data-category="code_exe"/, 'index.html must have Code Execution filter pill');
    assert.match(html, /class="[^"]*filter-pill[^"]*"[^>]*data-category="tool"/, 'index.html must have Tool Ops filter pill');
    assert.match(html, /class="[^"]*filter-pill[^"]*"[^>]*data-category="thought"/, 'index.html must have Thoughts filter pill');
    assert.match(html, /class="[^"]*filter-pill[^"]*"[^>]*data-category="error"/, 'index.html must have Errors filter pill');
  });

  test('VAL-SEARCH-002: Clicking category filter pills filters visible rows by message type', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, upsertLog } = harness.exports;

    upsertLog({ id: 'm-user', type: 'user', content: 'execute bash' });
    upsertLog({ id: 'm-code', type: 'code_exe', content: 'cargo build' });
    upsertLog({ id: 'm-tool', type: 'tool', content: 'read file.txt' });
    upsertLog({ id: 'm-thought', type: 'thought', thought: true, content: 'evaluating approach' });
    upsertLog({ id: 'm-error', type: 'error', content: 'command timed out' });

    // Open search drawer
    dom.searchToggleBtn.dispatch('click', {});

    // Filter by Code Execution
    dom.filterCodeBtn.dispatch('click', {});
    assert.equal(state.rows.get('m-code').hidden, false, 'm-code must be visible');
    assert.equal(state.rows.get('m-user').hidden, true, 'm-user must be hidden');
    assert.equal(state.rows.get('m-tool').hidden, true, 'm-tool must be hidden');
    assert.equal(state.rows.get('m-thought').hidden, true, 'm-thought must be hidden');
    assert.equal(state.rows.get('m-error').hidden, true, 'm-error must be hidden');
    assert.ok(dom.filterCodeBtn.classList.contains('active'), 'Code pill must have active class');

    // Filter by Tool Ops
    dom.filterToolBtn.dispatch('click', {});
    assert.equal(state.rows.get('m-tool').hidden, false, 'm-tool must be visible');
    assert.equal(state.rows.get('m-code').hidden, true, 'm-code must be hidden');
    assert.equal(state.rows.get('m-user').hidden, true, 'm-user must be hidden');
    assert.equal(state.rows.get('m-thought').hidden, true, 'm-thought must be hidden');
    assert.equal(state.rows.get('m-error').hidden, true, 'm-error must be hidden');

    // Filter by Thoughts
    dom.filterThoughtBtn.dispatch('click', {});
    assert.equal(state.rows.get('m-thought').hidden, false, 'm-thought must be visible');
    assert.equal(state.rows.get('m-tool').hidden, true, 'm-tool must be hidden');
    assert.equal(state.rows.get('m-code').hidden, true, 'm-code must be hidden');

    // Filter by Errors
    dom.filterErrorBtn.dispatch('click', {});
    assert.equal(state.rows.get('m-error').hidden, false, 'm-error must be visible');
    assert.equal(state.rows.get('m-thought').hidden, true, 'm-thought must be hidden');
    assert.equal(state.rows.get('m-tool').hidden, true, 'm-tool must be hidden');

    // Filter by All
    dom.filterAllBtn.dispatch('click', {});
    assert.equal(state.rows.get('m-user').hidden, false, 'm-user must be visible under All');
    assert.equal(state.rows.get('m-code').hidden, false, 'm-code must be visible under All');
    assert.equal(state.rows.get('m-tool').hidden, false, 'm-tool must be visible under All');
    assert.equal(state.rows.get('m-thought').hidden, false, 'm-thought must be visible under All');
    assert.equal(state.rows.get('m-error').hidden, false, 'm-error must be visible under All');
  });

  test('VAL-SEARCH-002: ui/chat.css styles search drawer, search matches, and active filter pills', () => {
    const css = readChatCss();

    assert.match(css, /\.search-drawer/, 'CSS must style .search-drawer');
    assert.match(css, /\.search-bar/, 'CSS must style .search-bar');
    assert.match(css, /\.search-input/, 'CSS must style .search-input');
    assert.match(css, /\.filter-pill/, 'CSS must style .filter-pill');
    assert.match(css, /\.filter-pill\.active/, 'CSS must style active filter-pill');
    assert.match(css, /mark\.search-match/, 'CSS must style search match highlights');
    assert.match(css, /mark\.search-match\.active/, 'CSS must style active search match');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-EXPORT-001: Transcript export to Markdown and JSON             */
/* ------------------------------------------------------------------ */

describe('VAL-EXPORT-001: Transcript export to Markdown and JSON', () => {
  test('VAL-EXPORT-001: ui/index.html exposes conversation export dropdown button and menu items', () => {
    const html = readIndexHtml();

    assert.match(html, /id="export-toggle-btn"/, 'ui/index.html must declare #export-toggle-btn');
    assert.match(html, /id="export-menu"/, 'ui/index.html must declare #export-menu');
    assert.match(html, /id="export-md-btn"/, 'ui/index.html must declare #export-md-btn');
    assert.match(html, /id="export-json-btn"/, 'ui/index.html must declare #export-json-btn');
    assert.match(html, /id="copy-transcript-btn"/, 'ui/index.html must declare #copy-transcript-btn');
  });

  test('VAL-EXPORT-001: clicking #export-toggle-btn toggles #export-menu dropdown visibility', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    assert.equal(dom.exportMenu.hidden, true, 'Export menu starts hidden');

    dom.exportToggleBtn.dispatch('click', {});
    assert.equal(dom.exportMenu.hidden, false, 'Clicking #export-toggle-btn opens export menu');

    dom.exportToggleBtn.dispatch('click', {});
    assert.equal(dom.exportMenu.hidden, true, 'Clicking again closes export menu');
  });

  test('VAL-EXPORT-001: Export Markdown triggers .md file download with structured messages and timestamps', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, upsertLog, exportMarkdown } = harness.exports;

    upsertLog({
      id: 'm-1',
      no: 1,
      type: 'user',
      content: 'Can you run the tests?',
      timestamp: 1700000000
    });
    upsertLog({
      id: 'm-2',
      no: 2,
      type: 'thought',
      thought: true,
      content: 'I need to check the test suite.',
      timestamp: 1700000001
    });
    upsertLog({
      id: 'm-3',
      no: 3,
      type: 'code_exe',
      content: 'cargo test',
      timestamp: 1700000002
    });
    upsertLog({
      id: 'm-4',
      no: 4,
      type: 'agent',
      content: 'All tests passed successfully.',
      timestamp: 1700000003
    });

    let downloadTriggered = false;
    let downloadedBlob = null;
    let downloadedFilename = '';

    const origCreateElement = harness.document.createElement;
    harness.document.createElement = (tag) => {
      const el = origCreateElement(tag);
      if (tag === 'a') {
        el.click = () => {
          downloadTriggered = true;
          downloadedFilename = el.download;
        };
      }
      return el;
    };

    harness.context.URL.createObjectURL = (blob) => {
      downloadedBlob = blob;
      return 'blob:mock-export-url';
    };
    harness.context.URL.revokeObjectURL = () => {};

    dom.exportMdBtn.dispatch('click', {});

    assert.equal(downloadTriggered, true, 'Export Markdown must trigger download');
    assert.match(downloadedFilename, /\.md$/, 'Filename must end with .md');
    assert.ok(downloadedBlob, 'Must create a Blob for download');

    const md = typeof exportMarkdown === 'function' ? exportMarkdown() : '';
    assert.match(md, /# (Conversation|Chat) Transcript/, 'Markdown must have a title');
    assert.match(md, /## (You|Assistant|User)/, 'Markdown must include role headings');
    assert.match(md, /Can you run the tests\?/, 'Markdown must contain user content');
    assert.match(md, /Thought/i, 'Markdown must include thought section');
    assert.match(md, /cargo test/, 'Markdown must include code execution');
    assert.match(md, /All tests passed successfully\./, 'Markdown must include agent content');
    assert.match(md, /2023|UTC|\d{4}/, 'Markdown must include timestamps');
  });

  test('VAL-EXPORT-001: Export JSON triggers .json download of raw logs', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, upsertLog, exportJson } = harness.exports;

    upsertLog({
      id: 'm-1',
      no: 1,
      type: 'user',
      content: 'Hello world',
      timestamp: 1700000000
    });
    upsertLog({
      id: 'm-2',
      no: 2,
      type: 'agent',
      content: 'Hello! How can I help?',
      timestamp: 1700000001
    });

    let downloadTriggered = false;
    let downloadedFilename = '';
    let downloadedBlob = null;

    const origCreateElement = harness.document.createElement;
    harness.document.createElement = (tag) => {
      const el = origCreateElement(tag);
      if (tag === 'a') {
        el.click = () => {
          downloadTriggered = true;
          downloadedFilename = el.download;
        };
      }
      return el;
    };

    harness.context.URL.createObjectURL = (blob) => {
      downloadedBlob = blob;
      return 'blob:mock-json-url';
    };
    harness.context.URL.revokeObjectURL = () => {};

    dom.exportJsonBtn.dispatch('click', {});

    assert.equal(downloadTriggered, true, 'Export JSON must trigger download');
    assert.match(downloadedFilename, /\.json$/, 'Filename must end with .json');

    const jsonStr = typeof exportJson === 'function' ? exportJson() : '';
    const parsed = JSON.parse(jsonStr);
    assert.ok(Array.isArray(parsed), 'Exported JSON must be an array of logs');
    assert.equal(parsed.length, 2, 'Exported JSON must contain all logs');
    assert.equal(parsed[0].id, 'm-1');
    assert.equal(parsed[0].content, 'Hello world');
    assert.equal(parsed[1].id, 'm-2');
  });

  test('VAL-EXPORT-001: ui/chat.css styles export dropdown and menu items', () => {
    const css = readChatCss();

    assert.match(css, /\.export-dropdown/, 'CSS must style .export-dropdown');
    assert.match(css, /\.export-menu/, 'CSS must style .export-menu');
    assert.match(css, /\.export-item/, 'CSS must style .export-item');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-EXPORT-002: Transcript clipboard copy                          */
/* ------------------------------------------------------------------ */

describe('VAL-EXPORT-002: Transcript clipboard copy', () => {
  test('VAL-EXPORT-002: Copy Transcript writes formatted transcript to navigator.clipboard and provides feedback', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, upsertLog } = harness.exports;

    upsertLog({
      id: 'm-1',
      no: 1,
      type: 'user',
      content: 'What is 2+2?',
      timestamp: 1700000000
    });
    upsertLog({
      id: 'm-2',
      no: 2,
      type: 'agent',
      content: '4',
      timestamp: 1700000001
    });

    let copiedText = null;
    harness.context.navigator = {
      clipboard: {
        writeText: (val) => {
          copiedText = val;
          return Promise.resolve();
        }
      }
    };

    dom.copyTranscriptBtn.dispatch('click', {});
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));

    assert.ok(copiedText, 'Clipboard writeText must be called');
    assert.match(copiedText, /What is 2\+2\?/, 'Transcript must contain user message');
    assert.match(copiedText, /4/, 'Transcript must contain assistant message');
    assert.match(dom.copyTranscriptBtn.textContent, /Copied!|Copied/i, 'Button text must show copied feedback');

    const reset = harness.timeouts.find((t) => t.delay === 1200 || t.delay === 1500 || t.delay === 2000);
    assert.ok(reset, 'Feedback reset timeout must be set');
    reset.fn();
    assert.match(dom.copyTranscriptBtn.textContent, /Copy Transcript/i, 'Button text must reset to Copy Transcript');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-PERF-001: Log rendering batch DocumentFragment optimization    */
/* ------------------------------------------------------------------ */

describe('VAL-PERF-001: Log rendering batch DocumentFragment optimization', () => {
  test('VAL-PERF-001: mergeLogs appends new logs via DocumentFragment in a single DOM mutation', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, mergeLogs, state } = harness.exports;

    let appendChildCalls = 0;
    let appendedFragments = 0;
    let appendedElements = 0;
    const origAppendChild = dom.messages.appendChild;
    dom.messages.appendChild = (child) => {
      appendChildCalls += 1;
      if (child && child.nodeType === 11) {
        appendedFragments += 1;
      } else {
        appendedElements += 1;
      }
      return origAppendChild.call(dom.messages, child);
    };

    let insertBeforeCalls = 0;
    const origInsertBefore = dom.messages.insertBefore;
    dom.messages.insertBefore = (newNode, ref) => {
      insertBeforeCalls += 1;
      return origInsertBefore.call(dom.messages, newNode, ref);
    };

    const batch = [];
    for (let i = 1; i <= 20; i += 1) {
      batch.push({
        id: `log-${i}`,
        no: i,
        type: i % 2 === 0 ? 'agent' : 'user',
        content: `Message ${i}`
      });
    }

    mergeLogs(batch);

    assert.equal(dom.messages.childElementCount, 20, 'All 20 rows must be present in dom.messages');
    assert.equal(state.order.length, 20, 'State order must track all 20 rows');
    assert.equal(appendedFragments, 1, 'Batch mergeLogs must append via DocumentFragment');
    assert.equal(appendChildCalls, 1, 'Batch mergeLogs must trigger single appendChild on dom.messages');
    assert.equal(insertBeforeCalls, 0, 'Initial load append must not trigger quadratic insertBefore calls');
  });

  test('VAL-PERF-001: mergeLogs preserves ascending numerical order even if input logs are unsorted', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, mergeLogs } = harness.exports;

    const unsortedBatch = [
      { id: 'msg-30', no: 30, type: 'agent', content: 'Thirty' },
      { id: 'msg-10', no: 10, type: 'user', content: 'Ten' },
      { id: 'msg-20', no: 20, type: 'agent', content: 'Twenty' }
    ];

    mergeLogs(unsortedBatch);

    assert.equal(dom.messages.childElementCount, 3);
    const orderNos = dom.messages.children.map((c) => c.dataset.id);
    assert.deepEqual(orderNos, ['msg-10', 'msg-20', 'msg-30'], 'Rows must be ordered by sequence/no ascending');
  });

  test('VAL-PERF-001: mergeLogs handles existing row updates and out-of-order incremental inserts', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, mergeLogs, state } = harness.exports;

    // Initial batch
    mergeLogs([
      { id: 'msg-10', no: 10, type: 'user', content: 'Ten' },
      { id: 'msg-30', no: 30, type: 'agent', content: 'Thirty' }
    ]);
    assert.equal(dom.messages.childElementCount, 2);

    // Merge update to msg-30 and new insert msg-20 which belongs between msg-10 and msg-30
    mergeLogs([
      { id: 'msg-30', no: 30, type: 'agent', content: 'Thirty updated' },
      { id: 'msg-20', no: 20, type: 'code_exe', content: 'Twenty inserted' }
    ]);

    assert.equal(dom.messages.childElementCount, 3);
    const orderNos = dom.messages.children.map((c) => c.dataset.id);
    assert.deepEqual(orderNos, ['msg-10', 'msg-20', 'msg-30']);
    const rec30 = state.logs.get('msg-30');
    assert.equal(rec30.content, 'Thirty updated');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-WORK-001 & VAL-WORK-002: Workspace file browser & file upload  */
/* ------------------------------------------------------------------ */

describe('VAL-WORK-001: Workspace file browser drawer rendering', () => {
  test('VAL-WORK-001: ui/index.html defines sidebar tabs and workspace drawer view', () => {
    const html = readIndexHtml();

    assert.match(html, /id="tab-sessions-btn"/, 'index.html must have #tab-sessions-btn');
    assert.match(html, /id="tab-workspace-btn"/, 'index.html must have #tab-workspace-btn');
    assert.match(html, /id="sessions-view"/, 'index.html must have #sessions-view');
    assert.match(html, /id="workspace-view"/, 'index.html must have #workspace-view');
    assert.match(html, /id="workspace-file-list"/, 'index.html must have #workspace-file-list');
    assert.match(html, /id="workspace-upload-dropzone"/, 'index.html must have #workspace-upload-dropzone');
  });

  test('VAL-WORK-001: sidebar tab switching toggles between sessions and workspace file views', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, switchSidebarTab } = harness.exports;

    assert.equal(dom.sessionsView.hidden, false, 'sessions-view starts visible');
    assert.equal(dom.workspaceView.hidden, true, 'workspace-view starts hidden');
    assert.ok(dom.tabSessionsBtn.classList.contains('active'), 'sessions tab active initially');

    dom.tabWorkspaceBtn.dispatch('click', {});
    assert.equal(dom.sessionsView.hidden, true, 'sessions-view hidden on workspace tab switch');
    assert.equal(dom.workspaceView.hidden, false, 'workspace-view visible on workspace tab switch');
    assert.ok(dom.tabWorkspaceBtn.classList.contains('active'), 'workspace tab active');
    assert.equal(state.sidebarTab, 'workspace');

    dom.tabSessionsBtn.dispatch('click', {});
    assert.equal(dom.sessionsView.hidden, false);
    assert.equal(dom.workspaceView.hidden, true);
    assert.equal(state.sidebarTab, 'sessions');
  });

  test('VAL-WORK-001: Workspace drawer displays the contents of the configured workspace directory with file sizes and modified timestamps', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, renderWorkspaceFiles, loadWorkspaceFiles } = harness.exports;

    const mockFiles = [
      {
        name: 'report.txt',
        path: 'report.txt',
        is_file: true,
        is_dir: false,
        size: 1048576, // 1 MB
        modified: 1700000000
      },
      {
        name: 'assets',
        path: 'assets',
        is_file: false,
        is_dir: true,
        size: 4096,
        modified: 1700003600
      }
    ];

    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_list') || u.includes('/api/workspace_files')) {
        return new Response(JSON.stringify({ ok: true, entries: mockFiles, count: 2 }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    await loadWorkspaceFiles();

    assert.equal(state.workspaceFiles.length, 2, 'state.workspaceFiles should store fetched files');
    assert.equal(dom.workspaceFileList.children.length, 2, 'File list DOM must contain 2 file entries');

    const fileItem = dom.workspaceFileList.children[0];
    assert.match(fileItem.innerHTML, /report\.txt/, 'File entry must show file name');
    assert.match(fileItem.innerHTML, /1\.0 MB|1 MB|1024 KB|1048576/, 'File entry must display formatted file size');
    assert.match(fileItem.innerHTML, /2023|UTC|\d{4}/, 'File entry must display modified timestamp');

    const dirItem = dom.workspaceFileList.children[1];
    assert.match(dirItem.innerHTML, /assets/, 'Directory entry must show directory name');
  });
});

describe('VAL-WORK-002: Workspace drag-and-drop file upload', () => {
  test('VAL-WORK-002: Drag and drop files into chat composer or workspace drawer uploads into workspace directory', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, uploadWorkspaceFiles } = harness.exports;

    const uploadedOps = [];
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_write') || u.includes('/api/upload_file') || u.includes('/api/write_file')) {
        uploadedOps.push(JSON.parse(opts.body));
        return new Response(JSON.stringify({ ok: true, success: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true, entries: [], count: 0 }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    // Test dropping onto workspace dropzone
    const fakeFile1 = {
      name: 'notes.md',
      size: 120,
      type: 'text/markdown',
      text: async () => '# Meeting notes'
    };

    let dropPrevented = false;
    dom.workspaceUploadDropzone.dispatch('drop', {
      preventDefault: () => {
        dropPrevented = true;
      },
      dataTransfer: {
        files: [fakeFile1]
      }
    });

    await new Promise((r) => setTimeout(r, 40));

    assert.equal(dropPrevented, true, 'drop event preventDefault must be called');
    assert.equal(uploadedOps.length, 1, 'Should call upload API for dropped file');
    assert.equal(uploadedOps[0].name || uploadedOps[0].path, 'notes.md', 'Uploaded file must match file name');

    // Test dropping onto composer
    const fakeFile2 = {
      name: 'data.json',
      size: 45,
      type: 'application/json',
      text: async () => '{"key":"value"}'
    };

    dom.composer.dispatch('drop', {
      preventDefault: () => {},
      dataTransfer: {
        files: [fakeFile2]
      }
    });

    await new Promise((r) => setTimeout(r, 20));

    assert.equal(uploadedOps.length, 2, 'Should call upload API for composer dropped file');
    assert.equal(uploadedOps[1].name || uploadedOps[1].path, 'data.json');
  });

  test('VAL-WORK-002: Dragover adds dragover/active class and dragleave removes it', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom } = harness.exports;

    let prevented = false;
    dom.workspaceUploadDropzone.dispatch('dragover', {
      preventDefault: () => {
        prevented = true;
      }
    });
    assert.equal(prevented, true, 'dragover preventDefault must be called');
    assert.ok(
      dom.workspaceUploadDropzone.classList.contains('dragover') ||
      dom.workspaceUploadDropzone.classList.contains('active'),
      'Dropzone must have dragover class on dragover'
    );

    dom.workspaceUploadDropzone.dispatch('dragleave', {});
    assert.equal(
      dom.workspaceUploadDropzone.classList.contains('dragover'),
      false,
      'Dropzone removes dragover class on dragleave'
    );
  });
});

/* ------------------------------------------------------------------ */
/* VAL-TERM-001: Collapsible bottom PTY terminal drawer & streaming   */
/* ------------------------------------------------------------------ */

describe('VAL-TERM-001: Interactive PTY terminal drawer streaming', () => {
  test('VAL-TERM-001: ui/index.html exposes terminal drawer, toggle button, output, and input controls', () => {
    const html = readIndexHtml();

    assert.match(html, /id="terminal-drawer"/, 'index.html must have #terminal-drawer');
    assert.match(html, /id="toggle-terminal-btn"/, 'index.html must have #toggle-terminal-btn');
    assert.match(html, /id="terminal-output"/, 'index.html must have #terminal-output');
    assert.match(html, /id="terminal-input"/, 'index.html must have #terminal-input');
    assert.match(html, /id="terminal-send-btn"/, 'index.html must have #terminal-send-btn');
    assert.match(html, /id="terminal-clear-btn"/, 'index.html must have #terminal-clear-btn');
    assert.match(html, /id="terminal-close-btn"/, 'index.html must have #terminal-close-btn');
  });

  test('VAL-TERM-001: Alt+T and #toggle-terminal-btn toggle #terminal-drawer and focus input', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state } = harness.exports;

    assert.equal(dom.terminalDrawer.hidden, true, 'Terminal drawer starts hidden');

    // Alt+T toggles drawer open
    let prevented = false;
    harness.document.fire('keydown', {
      altKey: true,
      key: 't',
      preventDefault: () => {
        prevented = true;
      }
    });

    assert.equal(prevented, true, 'Alt+T must call preventDefault()');
    assert.equal(dom.terminalDrawer.hidden, false, 'Alt+T opens #terminal-drawer');
    assert.equal(state.terminalOpen, true, 'state.terminalOpen must be true');

    // Alt+T toggles drawer closed
    harness.document.fire('keydown', {
      altKey: true,
      key: 't',
      preventDefault: () => {}
    });
    assert.equal(dom.terminalDrawer.hidden, true, 'Alt+T closes #terminal-drawer');
    assert.equal(state.terminalOpen, false, 'state.terminalOpen must be false');

    // Toggle button opens drawer
    dom.toggleTerminalBtn.dispatch('click', {});
    assert.equal(dom.terminalDrawer.hidden, false, 'toggle button opens drawer');
    assert.equal(state.terminalOpen, true);

    // Close button closes drawer
    dom.terminalCloseBtn.dispatch('click', {});
    assert.equal(dom.terminalDrawer.hidden, true, 'close button closes drawer');
    assert.equal(state.terminalOpen, false);
  });

  test('VAL-TERM-001: Active PTY output streams into terminal container in real time', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, appendTerminalOutput, clearTerminalOutput } = harness.exports;

    // Simulate streaming chunks from PTY
    appendTerminalOutput('$ ls -la\n');
    appendTerminalOutput('drwxr-xr-x 2 babim babim 4096 Nov 3 12:00 src\n');
    appendTerminalOutput('-rw-r--r-- 1 babim babim 1024 Nov 3 12:00 Cargo.toml\n');

    assert.match(dom.terminalOutput.textContent, /\$ ls -la/);
    assert.match(dom.terminalOutput.textContent, /Cargo\.toml/);

    // Test clear output
    clearTerminalOutput();
    assert.equal(dom.terminalOutput.textContent, '');
  });

  test('VAL-TERM-001: Bridge socket pty_output events stream directly into terminal container', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, connectSocket } = harness.exports;

    connectSocket();
    const ptyHandler = harness.listeners['pty_output'] || harness.listeners['terminal_output'] || harness.listeners['connector_exec_op_result'];
    assert.ok(ptyHandler, 'Socket listener for pty_output or terminal_output must be registered');

    ptyHandler({ output: 'Streaming log chunk 1\n' });
    assert.match(dom.terminalOutput.textContent, /Streaming log chunk 1/);

    ptyHandler('Streaming log chunk 2\n');
    assert.match(dom.terminalOutput.textContent, /Streaming log chunk 2/);
  });

  test('VAL-TERM-001: Manual command input sends command via exec API and appends echo', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, sendTerminalCommand } = harness.exports;

    let apiCalls = [];
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'csrf-token' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/exec')) {
        apiCalls.push(JSON.parse(opts.body));
        return new Response(JSON.stringify({ ok: true, output: 'Linux debian 6.1\n', exit_code: 0 }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    dom.terminalInput.value = 'uname -a';
    await sendTerminalCommand();

    assert.equal(apiCalls.length, 1, 'Should call /api/exec');
    assert.equal(apiCalls[0].command || apiCalls[0].code, 'uname -a');
    assert.match(dom.terminalOutput.textContent, /uname -a/, 'Output must show command prompt');
    assert.match(dom.terminalOutput.textContent, /Linux debian/, 'Output must show returned stream');
    assert.equal(dom.terminalInput.value, '', 'Input should be cleared after send');
  });

  test('VAL-TERM-001: ui/chat.css styles terminal drawer, output container, and prompt bar', () => {
    const css = readChatCss();

    assert.match(css, /\.terminal-drawer/, 'CSS must style .terminal-drawer');
    assert.match(css, /\.terminal-header/, 'CSS must style .terminal-header');
    assert.match(css, /\.terminal-output/, 'CSS must style .terminal-output');
    assert.match(css, /\.terminal-bar|\.terminal-prompt/, 'CSS must style prompt bar');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-STATUS-001: Topbar telemetry widget and latency indicator      */
/* ------------------------------------------------------------------ */

describe('VAL-STATUS-001: Topbar telemetry widget and latency indicator', () => {
  test('VAL-STATUS-001: ui/index.html exposes topbar status container and telemetry pill with RAM/CPU indicators', () => {
    const html = readIndexHtml();

    assert.match(
      html,
      /id="status-container"|class="[^"]*status-container[^"]*"/,
      'ui/index.html must expose topbar status container element'
    );
    assert.match(
      html,
      /id="telemetry-status-pill"|id="telemetry-pill"/,
      'ui/index.html must expose #telemetry-status-pill or #telemetry-pill'
    );
    assert.match(
      html,
      /id="telemetry-cpu"/,
      'ui/index.html must expose CPU indicator element'
    );
    assert.match(
      html,
      /id="telemetry-ram"/,
      'ui/index.html must expose RAM indicator element'
    );
    assert.match(
      html,
      /id="telemetry-latency"/,
      'ui/index.html must expose latency indicator element'
    );
    assert.match(
      html,
      /id="telemetry-agent-state"|id="telemetry-state"/,
      'ui/index.html must expose agent state indicator element'
    );
    assert.match(
      html,
      /id="diagnostics-cpu"/,
      'ui/index.html diagnostics modal must expose CPU metric row'
    );
    assert.match(
      html,
      /id="diagnostics-ram"/,
      'ui/index.html diagnostics modal must expose RAM metric row'
    );
  });

  test('VAL-STATUS-001: ui/container.js renders compact telemetry pill with live RAM/CPU, ping latency, and agent state', () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, createElement } = harness;

    const statusContainer = createElement('status-container');
    const pill = createElement('telemetry-status-pill');
    const cpuEl = createElement('telemetry-cpu');
    const cpuValEl = createElement('telemetry-cpu-val');
    const ramEl = createElement('telemetry-ram');
    const ramValEl = createElement('telemetry-ram-val');
    const latencyEl = createElement('telemetry-latency');
    const latencyValEl = createElement('telemetry-latency-val');
    const stateEl = createElement('telemetry-agent-state');

    elements['status-container'] = statusContainer;
    elements['telemetry-status-pill'] = pill;
    elements['telemetry-cpu'] = cpuEl;
    elements['telemetry-cpu-val'] = cpuValEl;
    elements['telemetry-ram'] = ramEl;
    elements['telemetry-ram-val'] = ramValEl;
    elements['telemetry-latency'] = latencyEl;
    elements['telemetry-latency-val'] = latencyValEl;
    elements['telemetry-agent-state'] = stateEl;

    bridge.installDiagnostics(document);

    assert.equal(typeof bridge.getTelemetryMetrics, 'function', 'bridge.getTelemetryMetrics must be function');
    assert.equal(typeof bridge.updateTelemetry, 'function', 'bridge.updateTelemetry must be function');

    // Update telemetry metrics with RAM, CPU, latency
    bridge.updateTelemetry({
      cpu: 24,
      ram: 512,
      latency: 16
    });

    const metrics = bridge.getTelemetryMetrics();
    assert.equal(metrics.cpu, '24%', 'CPU metric must format to 24%');
    assert.equal(metrics.ram, '512 MB', 'RAM metric must format to 512 MB');
    assert.equal(metrics.latency, '16 ms', 'Latency metric must format to 16 ms');
    assert.equal(metrics.agentState, 'Idle', 'Initial agent state must be Idle');

    assert.equal(cpuValEl.textContent, '24%');
    assert.equal(ramValEl.textContent, '512 MB');
    assert.equal(latencyValEl.textContent, '16 ms');
    assert.equal(stateEl.textContent, 'Idle');

    // State changes: busy thinking
    bridge.setTaskState(true, 'Thinking about next steps');
    assert.equal(bridge.getTelemetryMetrics().agentState, 'Thinking', 'Agent state must be Thinking when summary mentions think');
    assert.equal(stateEl.textContent, 'Thinking');

    // State changes: busy executing
    bridge.setTaskState(true, 'Executing shell command');
    assert.equal(bridge.getTelemetryMetrics().agentState, 'Executing', 'Agent state must be Executing when active');
    assert.equal(stateEl.textContent, 'Executing');

    // State changes: idle
    bridge.setTaskState(false);
    assert.equal(bridge.getTelemetryMetrics().agentState, 'Idle', 'Agent state must return to Idle when task completes');
    assert.equal(stateEl.textContent, 'Idle');
  });

  test('VAL-STATUS-001: clicking telemetry status pill opens diagnostics details modal', () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, createElement } = harness;

    const pill = createElement('telemetry-status-pill');
    const modal = createElement('diagnostics-modal');
    modal.hidden = true;
    const cpuEl = createElement('diagnostics-cpu');
    const ramEl = createElement('diagnostics-ram');
    const agentStateEl = createElement('diagnostics-agent-state');

    elements['telemetry-status-pill'] = pill;
    elements['diagnostics-modal'] = modal;
    elements['diagnostics-cpu'] = cpuEl;
    elements['diagnostics-ram'] = ramEl;
    elements['diagnostics-agent-state'] = agentStateEl;

    bridge.installDiagnostics(document);

    bridge.updateTelemetry({
      cpu: 35,
      ram: 1024,
      latency: 22
    });

    assert.equal(bridge.isDiagnosticsModalOpen(), false, 'modal should initially be closed');

    // Clicking telemetry pill opens modal
    pill.dispatch('click', { preventDefault: () => {} });

    assert.equal(bridge.isDiagnosticsModalOpen(), true, 'modal must be open after pill click');
    assert.equal(modal.hidden, false, 'modal.hidden must be false');
    assert.equal(cpuEl.textContent, '35%');
    assert.equal(ramEl.textContent, '1024 MB');
  });

  test('VAL-STATUS-001: WebSocket telemetry op result updates host RAM, CPU, and ping latency', () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, createElement } = harness;

    const cpuValEl = createElement('telemetry-cpu-val');
    const ramValEl = createElement('telemetry-ram-val');
    elements['telemetry-cpu-val'] = cpuValEl;
    elements['telemetry-ram-val'] = ramValEl;

    bridge.installDiagnostics(document);

    // Simulate incoming connector_telemetry_op_result packet over WebSocket
    const packet = '42["connector_telemetry_op_result", {' +
      '"type": "connector_telemetry_op_result",' +
      '"op": "connector_telemetry_op",' +
      '"id": "tel-123",' +
      '"cpu_count": 8,' +
      '"cpu_usage_percent": 42.8,' +
      '"total_ram_mb": 16384,' +
      '"available_ram_mb": 12288' +
    '}]';

    bridge.handleWebSocketMessage(packet);

    const metrics = bridge.getTelemetryMetrics();
    assert.equal(metrics.cpu, '43%', 'CPU percentage rounded from 42.8% to 43%');
    assert.equal(metrics.ram, '4096 MB', 'RAM used is total (16384) - available (12288) = 4096 MB');
    assert.equal(cpuValEl.textContent, '43%');
    assert.equal(ramValEl.textContent, '4096 MB');
  });

  test('VAL-STATUS-001: ui/chat.css styles topbar telemetry pill, state badges, and indicators', () => {
    const css = readChatCss();

    assert.match(css, /\.status-container/, 'CSS must style .status-container');
    assert.match(css, /\.telemetry-status-pill|\.telemetry-pill/, 'CSS must style .telemetry-status-pill');
    assert.match(css, /\.telemetry-agent-state/, 'CSS must style .telemetry-agent-state');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-FIX-UI-001: Chat socket auth callback retry and error handling */
/* ------------------------------------------------------------------ */

describe('VAL-FIX-UI-001: Chat socket auth callback retry and error handling', () => {
  test('VAL-FIX-UI-001: refreshCsrf retries with small backoff on transient failure and succeeds', async () => {
    const harness = createHarness();
    const { refreshCsrf, state } = harness.exports;
    state.base = 'http://localhost:3000';

    let calls = 0;
    harness.context.fetch = async () => {
      calls++;
      if (calls === 1) {
        return { status: 502, ok: false };
      }
      return {
        status: 200,
        ok: true,
        json: async () => ({ ok: true, token: 'csrf-retry-success' })
      };
    };

    const token = await refreshCsrf();
    assert.equal(token, 'csrf-retry-success', 'must return token after transient retry');
    assert.equal(calls, 2, 'must have retried once after 502');
  });

  test('VAL-FIX-UI-001: refreshCsrf retries with backoff and fails after max attempts on persistent error', async () => {
    const harness = createHarness();
    const { refreshCsrf, state } = harness.exports;
    state.base = 'http://localhost:3000';

    let calls = 0;
    harness.context.fetch = async () => {
      calls++;
      return { status: 502, ok: false };
    };

    await assert.rejects(
      async () => {
        await refreshCsrf();
      },
      /CSRF bootstrap failed/i
    );
    assert.equal(calls, 3, 'must attempt 3 times (1 initial + 2 retries) before failing');
  });

  test('VAL-FIX-UI-001: refreshCsrf does not retry on 401/403 auth errors', async () => {
    const harness = createHarness();
    const { refreshCsrf, state } = harness.exports;
    state.base = 'http://localhost:3000';

    let calls = 0;
    harness.context.fetch = async () => {
      calls++;
      return { status: 401, ok: false };
    };

    await assert.rejects(
      async () => {
        await refreshCsrf();
      },
      /CSRF bootstrap failed \(HTTP 401\)/i
    );
    assert.equal(calls, 1, 'must fail immediately without retrying on 401');
  });

  test('VAL-FIX-UI-001: connectSocket auth callback passes error object instead of empty packet on persistent CSRF failure', async () => {
    const harness = createHarness();
    const { connectSocket, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.everConnected = true;
    state.expired = false;

    let calls = 0;
    harness.context.fetch = async () => {
      calls++;
      return { status: 502, ok: false };
    };

    connectSocket();
    const authFn = harness.mockIo.authOpts.auth;
    assert.ok(typeof authFn === 'function');

    const authData = await new Promise((resolve) => {
      authFn((data) => {
        resolve(data);
      });
    });

    assert.equal(calls, 3, 'must retry 3 times before calling auth callback with error');
    assert.notDeepEqual(authData, {}, 'auth callback must not receive empty object');
    assert.ok(
      authData instanceof Error || (authData && typeof authData.message === 'string'),
      'auth callback must receive an error object'
    );
    assert.match(authData.message, /CSRF bootstrap failed/i);
  });

  test('VAL-FIX-UI-001: connectSocket auth callback passes csrf_token and handlers on success', async () => {
    const harness = createHarness();
    const { connectSocket, state } = harness.exports;
    state.base = 'http://localhost:3000';
    state.everConnected = true;

    harness.context.fetch = async () => ({
      status: 200,
      ok: true,
      json: async () => ({ ok: true, token: 'valid-handshake-token' })
    });

    connectSocket();
    const authFn = harness.mockIo.authOpts.auth;
    assert.ok(typeof authFn === 'function');

    const authData = await new Promise((resolve) => {
      authFn((data) => {
        resolve(data);
      });
    });

    assert.equal(authData.csrf_token, 'valid-handshake-token');
    assert.deepEqual(Array.from(authData.handlers), ['ws_webui']);
  });
});

/* ------------------------------------------------------------------ */
/* VAL-DEBT-003: Binary file upload safe transfer                     */
/* ------------------------------------------------------------------ */

describe('VAL-DEBT-003: Binary file upload safe transfer', () => {
  test('VAL-DEBT-003: isBinaryFile accurately detects binary file types and extensions vs text files', () => {
    const harness = createHarness();
    const { isBinaryFile } = harness.exports;

    assert.equal(typeof isBinaryFile, 'function', 'isBinaryFile must be exported');

    // Binary file extensions
    assert.equal(isBinaryFile({ name: 'image.png' }), true, 'PNG should be binary');
    assert.equal(isBinaryFile({ name: 'photo.jpg' }), true, 'JPG should be binary');
    assert.equal(isBinaryFile({ name: 'photo.jpeg' }), true, 'JPEG should be binary');
    assert.equal(isBinaryFile({ name: 'archive.zip' }), true, 'ZIP should be binary');
    assert.equal(isBinaryFile({ name: 'archive.tar.gz' }), true, 'TAR.GZ should be binary');
    assert.equal(isBinaryFile({ name: 'manual.pdf' }), true, 'PDF should be binary');
    assert.equal(isBinaryFile({ name: 'program.exe' }), true, 'EXE should be binary');
    assert.equal(isBinaryFile({ name: 'raw.bin' }), true, 'BIN should be binary');
    assert.equal(isBinaryFile({ name: 'module.wasm' }), true, 'WASM should be binary');
    assert.equal(isBinaryFile({ name: 'track.mp3' }), true, 'MP3 should be binary');

    // Binary MIME types
    assert.equal(isBinaryFile({ name: 'unknown', type: 'image/png' }), true, 'image/png MIME should be binary');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'application/pdf' }), true, 'application/pdf MIME should be binary');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'application/zip' }), true, 'application/zip MIME should be binary');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'application/octet-stream' }), true, 'application/octet-stream MIME should be binary');

    // Text file extensions
    assert.equal(isBinaryFile({ name: 'notes.txt' }), false, 'TXT should be text');
    assert.equal(isBinaryFile({ name: 'readme.md' }), false, 'MD should be text');
    assert.equal(isBinaryFile({ name: 'data.json' }), false, 'JSON should be text');
    assert.equal(isBinaryFile({ name: 'script.js' }), false, 'JS should be text');
    assert.equal(isBinaryFile({ name: 'index.html' }), false, 'HTML should be text');
    assert.equal(isBinaryFile({ name: 'style.css' }), false, 'CSS should be text');
    assert.equal(isBinaryFile({ name: 'main.rs' }), false, 'RS should be text');
    assert.equal(isBinaryFile({ name: 'graphic.svg' }), false, 'SVG should be text');
    assert.equal(isBinaryFile({ name: 'config.yaml' }), false, 'YAML should be text');

    // Text MIME types
    assert.equal(isBinaryFile({ name: 'unknown', type: 'text/plain' }), false, 'text/plain MIME should be text');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'application/json' }), false, 'application/json MIME should be text');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'text/html' }), false, 'text/html MIME should be text');
    assert.equal(isBinaryFile({ name: 'unknown', type: 'image/svg+xml' }), false, 'image/svg+xml MIME should be text');
  });

  test('VAL-DEBT-003: uploadSingleFile transfers binary files safely via base64 encoding without UTF-8 corruption', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { uploadSingleFile } = harness.exports;

    assert.equal(typeof uploadSingleFile, 'function', 'uploadSingleFile must be exported');

    let uploadedPayload = null;
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_write')) {
        uploadedPayload = JSON.parse(opts.body);
        return new Response(JSON.stringify({ ok: true, success: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    // Construct arbitrary binary bytes with non-UTF8 and null bytes: 0x89 PNG header, 0xFF, 0x00, 0xFE
    const binaryBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0xff, 0xfe, 0x00]);
    const expectedBase64 = Buffer.from(binaryBytes).toString('base64');

    const fakeBinaryFile = {
      name: 'screenshot.png',
      size: binaryBytes.length,
      type: 'image/png',
      arrayBuffer: async () => binaryBytes.buffer
    };

    await uploadSingleFile(fakeBinaryFile);

    assert.ok(uploadedPayload, 'files_write API must be called');
    assert.equal(uploadedPayload.name, 'screenshot.png');
    assert.equal(uploadedPayload.path, 'screenshot.png');
    assert.equal(uploadedPayload.encoding, 'base64', 'Binary file upload must specify base64 encoding');
    assert.equal(uploadedPayload.content, expectedBase64, 'Binary content must match base64 encoded bytes');
    assert.equal(uploadedPayload.size, binaryBytes.length);
  });

  test('VAL-DEBT-003: uploadSingleFile continues to upload text files as plaintext', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { uploadSingleFile } = harness.exports;

    let uploadedPayload = null;
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_write')) {
        uploadedPayload = JSON.parse(opts.body);
        return new Response(JSON.stringify({ ok: true, success: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    const textContent = '# Project Plan\n\n- Step 1: Tests\n- Step 2: Implementation\n\nUnicode: 🚀 Tiếng Việt';
    const fakeTextFile = {
      name: 'plan.md',
      size: Buffer.byteLength(textContent, 'utf8'),
      type: 'text/markdown',
      text: async () => textContent
    };

    await uploadSingleFile(fakeTextFile);

    assert.ok(uploadedPayload, 'files_write API must be called');
    assert.equal(uploadedPayload.name, 'plan.md');
    assert.equal(uploadedPayload.path, 'plan.md');
    assert.equal(uploadedPayload.content, textContent, 'Text file must transfer exact plaintext');
    assert.ok(
      uploadedPayload.encoding === 'utf-8' || uploadedPayload.encoding === undefined,
      'Text file should have utf-8 or default encoding'
    );
  });

  test('VAL-DEBT-003: Workspace drag-and-drop handles mixed binary and text files', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, uploadWorkspaceFiles } = harness.exports;

    const uploadedOps = [];
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_write')) {
        uploadedOps.push(JSON.parse(opts.body));
        return new Response(JSON.stringify({ ok: true, success: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_list') || u.includes('/api/workspace_files')) {
        return new Response(JSON.stringify({ ok: true, entries: [], count: 0 }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    const zipBytes = new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00]);
    const zipBase64 = Buffer.from(zipBytes).toString('base64');
    const fakeZip = {
      name: 'archive.zip',
      size: zipBytes.length,
      type: 'application/zip',
      arrayBuffer: async () => zipBytes.buffer
    };

    const fakeJson = {
      name: 'config.json',
      size: 20,
      type: 'application/json',
      text: async () => '{"debug": true}'
    };

    dom.workspaceUploadDropzone.dispatch('drop', {
      preventDefault: () => {},
      dataTransfer: {
        files: [fakeZip, fakeJson]
      }
    });

    await new Promise((r) => setTimeout(r, 40));

    assert.equal(uploadedOps.length, 2, 'Should upload both files');
    assert.equal(uploadedOps[0].name, 'archive.zip');
    assert.equal(uploadedOps[0].encoding, 'base64');
    assert.equal(uploadedOps[0].content, zipBase64);

    assert.equal(uploadedOps[1].name, 'config.json');
    assert.equal(uploadedOps[1].content, '{"debug": true}');
  });
});

describe('VAL-DEBT-004: Tooling and CI hygiene', () => {
  test('VAL-DEBT-004: tools/tests/tray_rig_restart_test.sh uses relative RIG path without hardcoded /home/babim', () => {
    const shPath = new URL('tray_rig_restart_test.sh', import.meta.url);
    const sh = fs.readFileSync(shPath, 'utf8');

    assert.doesNotMatch(
      sh,
      /\/home\/babim/,
      'tray_rig_restart_test.sh must not contain hardcoded user path /home/babim'
    );
    assert.doesNotMatch(
      sh,
      /\/home\/[a-zA-Z0-9_-]+/,
      'tray_rig_restart_test.sh must not contain hardcoded /home/<user> paths'
    );
    assert.match(
      sh,
      /RIG=(?:(?:\$\{[^}]*:-)?["']?\$\(dirname ["']?\$\{?BASH_SOURCE\[0\]\}?["']?\)\/\.\.\/tray_rig\.sh["']?|\.\.\/tray_rig\.sh)/,
      'tray_rig_restart_test.sh must use relative RIG path to tray_rig.sh'
    );
  });

  test('VAL-DEBT-004: tray_rig.sh and restart test search for window with regex matching AQVN.*Workspace', () => {
    const testShPath = new URL('tray_rig_restart_test.sh', import.meta.url);
    const testSh = fs.readFileSync(testShPath, 'utf8');
    const rigShPath = new URL('../tray_rig.sh', import.meta.url);
    const rigSh = fs.readFileSync(rigShPath, 'utf8');

    assert.match(
      testSh,
      /xdotool\s+search\s+--onlyvisible\s+--name\s+['"]AQVN\.\*Workspace['"]/,
      'tray_rig_restart_test.sh must search for window matching AQVN.*Workspace regex'
    );
    assert.doesNotMatch(
      testSh,
      /xdotool\s+search\s+--onlyvisible\s+--name\s+['"]AQVN AI Workspace['"]/,
      'tray_rig_restart_test.sh must not use static AQVN AI Workspace string'
    );

    assert.match(
      rigSh,
      /xdotool\s+search\s+--onlyvisible\s+--name\s+['"]AQVN\.\*Workspace['"]/,
      'tray_rig.sh must search for window matching AQVN.*Workspace regex'
    );
    assert.doesNotMatch(
      rigSh,
      /xdotool\s+search\s+--onlyvisible\s+--name\s+['"]AQVN AI Workspace['"]/,
      'tray_rig.sh must not use static AQVN AI Workspace string'
    );
  });

  test('VAL-DEBT-004: tools/benchmark.sh sets MAX_BINARY_BYTES to 10485760 (10MB)', () => {
    const benchPath = new URL('../benchmark.sh', import.meta.url);
    const bench = fs.readFileSync(benchPath, 'utf8');

    assert.match(
      bench,
      /MAX_BINARY_BYTES=10485760/,
      'benchmark.sh must set MAX_BINARY_BYTES to 10485760'
    );
    assert.doesNotMatch(
      bench,
      /MAX_BINARY_BYTES=8388608/,
      'benchmark.sh must not retain legacy 8MB threshold'
    );
  });

  test('VAL-DEBT-004: .github/workflows/ci.yml runs tests, clippy, and fmt on master push and PRs', () => {
    const ciPath = new URL('../../.github/workflows/ci.yml', import.meta.url);
    assert.ok(fs.existsSync(ciPath), '.github/workflows/ci.yml must exist');

    const ciYaml = fs.readFileSync(ciPath, 'utf8');
    assert.match(ciYaml, /push:\s*\n\s+branches:\s*\n\s+-\s+master|push:\s*\n\s+branches:\s*\[.*master.*\]/, 'CI must trigger on master push');
    assert.match(ciYaml, /pull_request/, 'CI must trigger on pull requests');
    assert.match(ciYaml, /cargo\s+test/, 'CI must run cargo test');
    assert.match(ciYaml, /cargo\s+clippy/, 'CI must run cargo clippy');
    assert.match(ciYaml, /cargo\s+fmt/, 'CI must run cargo fmt');
  });
});

describe('VAL-DEBT-005: OpenSpec completed changes archive', () => {
  test('VAL-DEBT-005: openspec/changes has zero lingering active change129-change161 directories', () => {
    const changesDir = new URL('../../openspec/changes', import.meta.url);
    const active = fs.readdirSync(changesDir).filter((f) => {
      const match = f.match(/^change(\d+)-/);
      if (!match) return false;
      const num = parseInt(match[1], 10);
      return num >= 129 && num <= 161;
    });
    assert.equal(active.length, 0, `Expected zero lingering change129-161 directories in openspec/changes, found: ${active.join(', ')}`);
  });

  test('VAL-DEBT-005: archived changes exist in openspec/changes/archive/ with tasks marked complete', () => {
    const archiveDir = new URL('../../openspec/changes/archive', import.meta.url);
    assert.ok(fs.existsSync(archiveDir), 'openspec/changes/archive must exist');
    const archived = fs.readdirSync(archiveDir);

    for (let i = 129; i <= 161; i++) {
      const match = archived.find((f) => f.includes(`change${i}-`));
      assert.ok(match, `change${i} must be archived in openspec/changes/archive`);

      const tasksMdPath = new URL(`../../openspec/changes/archive/${match}/tasks.md`, import.meta.url);
      if (fs.existsSync(tasksMdPath)) {
        const content = fs.readFileSync(tasksMdPath, 'utf8');
        assert.doesNotMatch(content, /^- \[ \] /m, `tasks.md in ${match} must not contain unchecked tasks`);
      }
    }
  });
});

describe('VAL-SEC-006: Proxy query parameter open-redirect protection', () => {
  test('VAL-SEC-006: non-loopback proxy query param is rejected', () => {
    const chatJsPath = new URL('../../ui/chat.js', import.meta.url);
    const content = fs.readFileSync(chatJsPath, 'utf8');
    assert.match(content, /parsed\.hostname === 'localhost' \|\| parsed\.hostname === '127\.0\.0\.1' \|\| parsed\.hostname === '::1'/);
  });
});

describe('Milestone 2: Workspace File Explorer 2.0 (VAL-WORK-001..005)', () => {
  test('VAL-WORK-001: Hierarchical directory navigation & breadcrumbs', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="workspace-breadcrumbs"/, 'Breadcrumbs container must exist in index.html');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function renderBreadcrumbs\(\)/, 'renderBreadcrumbs must be defined in chat.js');
    assert.match(chatJs, /breadcrumb-item/, 'breadcrumb item styling used');
  });

  test('VAL-WORK-002 & VAL-WORK-003: File preview modal and inline editor', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="file-preview-modal"/, 'file-preview-modal must exist');
    assert.match(indexHtml, /id="file-editor-textarea"/, 'file-editor-textarea must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function openFilePreview\(/, 'openFilePreview must be defined');
    assert.match(chatJs, /function toggleFileEditor\(\)/, 'toggleFileEditor must be defined');
    assert.match(chatJs, /function saveFileEditor\(\)/, 'saveFileEditor must be defined');
  });

  test('VAL-WORK-004: Workspace CRUD buttons and ZIP archive download', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="workspace-new-file-btn"/, 'New file button must exist');
    assert.match(indexHtml, /id="workspace-new-folder-btn"/, 'New folder button must exist');
    assert.match(indexHtml, /id="workspace-zip-btn"/, 'ZIP download button must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /createWorkspaceFile/, 'createWorkspaceFile must exist');
    assert.match(chatJs, /createWorkspaceFolder/, 'createWorkspaceFolder must exist');
    assert.match(chatJs, /deleteWorkspaceItem/, 'deleteWorkspaceItem must exist');
    assert.match(chatJs, /downloadWorkspaceZip/, 'downloadWorkspaceZip must exist');
  });

  test('VAL-WORK-005: Visual diff / patch viewer', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="diff-viewer-modal"/, 'diff-viewer-modal must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function openDiffViewer\(/, 'openDiffViewer must be defined');
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.diff-line-add/, 'diff addition styling exists');
    assert.match(chatCss, /\.diff-line-del/, 'diff deletion styling exists');
  });
});

describe('Milestone 3: Interactive PTY Terminal Engine 2.0 (VAL-TERM-001..005)', () => {
  test('VAL-TERM-001: ANSI VT100 color parser', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function parseAnsiToHtml\(/, 'parseAnsiToHtml must be defined in chat.js');
    assert.match(chatJs, /ansi-fg-red/, 'ANSI color map contains standard colors');
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.ansi-fg-red/, 'CSS defines ANSI color classes');
  });

  test('VAL-TERM-002: Bi-directional streaming stdin', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /sendTerminalCommand/, 'Terminal input sends command via exec');
    assert.match(chatJs, /appendTerminalOutput/, 'Terminal appends output stream');
  });

  test('VAL-TERM-003 & VAL-TERM-004: Command history ring-buffer and resize', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /handleTerminalKeydown/, 'handleTerminalKeydown handles ArrowUp and ArrowDown');
    assert.match(chatJs, /emitTerminalResize/, 'emitTerminalResize calculates cols and rows');
  });

  test('VAL-TERM-005: Multi-tab terminal shell management', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="terminal-tabs"/, 'terminal-tabs container must exist');
    assert.match(indexHtml, /id="terminal-add-tab-btn"/, 'terminal-add-tab-btn must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /renderTerminalTabs/, 'renderTerminalTabs function must exist');
    assert.match(chatJs, /switchTerminalTab/, 'switchTerminalTab function must exist');
    assert.match(chatJs, /addTerminalTab/, 'addTerminalTab function must exist');
  });
});

describe('Milestone 4: Agent Security & Approval Gate (VAL-AGENT-001..004)', () => {
  test('VAL-AGENT-001: Real-time agent activity drawer and badge', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="agent-activity-drawer"/, 'agent-activity-drawer must exist');
    assert.match(indexHtml, /id="activity-badge-count"/, 'activity-badge-count must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function recordAgentActivity\(/, 'recordAgentActivity must be defined');
    assert.match(chatJs, /function toggleActivityDrawer\(\)/, 'toggleActivityDrawer must be defined');
  });

  test('VAL-AGENT-002: Human-in-the-loop action approval modal', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="agent-approval-modal"/, 'agent-approval-modal must exist');
    assert.match(indexHtml, /id="approval-deny-btn"/, 'approval-deny-btn must exist');
    assert.match(indexHtml, /id="approval-allow-once-btn"/, 'approval-allow-once-btn must exist');
    assert.match(indexHtml, /id="approval-allow-session-btn"/, 'approval-allow-session-btn must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function requestActionApproval\(/, 'requestActionApproval must return Promise');
  });

  test('VAL-AGENT-003: Security policy presets in settings', () => {
    const settingsHtml = fs.readFileSync(new URL('../../ui/settings.html', import.meta.url), 'utf8');
    assert.match(settingsHtml, /id="security-policy"/, 'security-policy select must exist in settings.html');
    assert.match(settingsHtml, /value="supervised"/, 'supervised preset exists');
    assert.match(settingsHtml, /value="permissive"/, 'permissive preset exists');
    assert.match(settingsHtml, /value="strict"/, 'strict preset exists');
    const settingsJs = fs.readFileSync(new URL('../../ui/settings.js', import.meta.url), 'utf8');
    assert.match(settingsJs, /security_policy/, 'security_policy stored in settings');
  });

  test('VAL-AGENT-004: Immutable audit trail recording and export', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="audit-export-json-btn"/, 'Export JSON button must exist');
    assert.match(indexHtml, /id="audit-export-csv-btn"/, 'Export CSV button must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function exportAuditJson\(\)/, 'exportAuditJson must be defined');
    assert.match(chatJs, /function exportAuditCsv\(\)/, 'exportAuditCsv must be defined');
  });
});

describe('Milestone 5: Modern UI/UX Polish & Multi-Server Gateway Radar (VAL-UI-001..006, VAL-CROSS-001..002)', () => {
  test('VAL-UI-001: Toast notification system', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="toast-container"/, 'toast-container must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function showToast\(/, 'showToast function must be defined');
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.toast/, 'CSS styles toast element');
    assert.match(chatCss, /\.toast-success/, 'CSS styles success toast variant');
  });

  test('VAL-UI-002: Smart scroll anchoring and bottom jump button', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="scroll-bottom-btn"/, 'scroll-bottom-btn must exist in chat pane');
    assert.match(indexHtml, /id="unread-count"/, 'unread-count badge must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function handleMessagesScroll\(\)/, 'handleMessagesScroll must be defined');
    assert.match(chatJs, /state\.userScrolledUp/, 'userScrolledUp flag tracks scroll state');
  });

  test('VAL-UI-003: Message action bar and collapsible code blocks', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function attachMessageActions\(/, 'attachMessageActions must be defined');
    assert.match(chatJs, /code-collapse-btn/, 'code collapse button attached for blocks >50 lines');
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.message-action-bar/, 'CSS styles message-action-bar');
    assert.match(chatCss, /\.code-collapse-btn/, 'CSS styles code-collapse-btn');
  });

  test('VAL-UI-004: Command palette and keyboard shortcuts modal', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="command-palette-modal"/, 'command-palette-modal must exist');
    assert.match(indexHtml, /id="shortcuts-modal"/, 'shortcuts-modal must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function openCommandPalette\(\)/, 'openCommandPalette must be defined');
    assert.match(chatJs, /function openShortcutsModal\(\)/, 'openShortcutsModal must be defined');
    assert.match(chatJs, /COMMAND_PALETTE_ACTIONS/, 'Palette actions inventory defined');
  });

  test('VAL-UI-005: Topbar fast server profile switcher', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="active-server-btn"/, 'active-server-btn must exist in topbar');
    assert.match(indexHtml, /id="server-profile-dropdown"/, 'server-profile-dropdown must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function renderServerDropdownList\(\)/, 'renderServerDropdownList must be defined');
    assert.match(chatJs, /function switchActiveServer\(/, 'switchActiveServer must be defined');
  });

  test('VAL-UI-006: Connection health radar and latency sparkline', () => {
    const indexHtml = fs.readFileSync(new URL('../../ui/index.html', import.meta.url), 'utf8');
    assert.match(indexHtml, /id="latency-sparkline"/, 'latency-sparkline element must exist');
    assert.match(indexHtml, /id="diagnostics-quality"/, 'diagnostics-quality element must exist');
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /function updateLatencySparkline\(/, 'updateLatencySparkline must be defined');
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.latency-sparkline/, 'CSS styles latency sparkline');
    assert.match(chatCss, /\.sparkline-bar/, 'CSS styles sparkline bars');
  });

  test('VAL-CROSS-001 & VAL-CROSS-002: Cross-area integration workflows', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /recordAgentActivity/, 'Agent activity tracking wired across flows');
    assert.match(chatJs, /requestActionApproval/, 'Approval gate available to agent flows');
    assert.match(chatJs, /renderTerminalTabs/, 'Terminal tab state preserved');
  });
});

describe('Phase 2 Hardening: Outbox, Terminal memory, a11y & preview sanitization', () => {
  test('Outbox deduplicates identical message_id on queue insertion', () => {
    const harness = createHarness();
    harness.exports.init();
    const { queueOutbox, readOutbox } = harness.exports;

    queueOutbox('first text', 'msg-dup-1');
    const firstCount = readOutbox().length;
    queueOutbox('first text again', 'msg-dup-1');
    const secondCount = readOutbox().length;

    assert.equal(firstCount, secondCount, 'Duplicate message_id must not add extra entry to outbox');
  });

  test('Terminal output limits scrollback to 2,000 lines and provides tab closing', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /lines\.slice\(-2000\)\.join\('\\n'\)/, 'Scrollback capped at 2000 lines');
    assert.match(chatJs, /function closeTerminalTab\(/, 'closeTerminalTab must be defined');
    assert.match(chatJs, /terminal-tab-close/, 'Tab close button rendered');
  });

  test('Subfolder uploads preserve active workspace path', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /state\.workspaceCurrentPath\s*\?[\s\S]*?state\.workspaceCurrentPath[\s\S]*?name\s*:\s*name/, 'Upload path scopes into subfolder');
  });

  test('Command palette supports Arrow navigation and option roles', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /setAttribute\('role',\s*'option'\)/, 'Palette items have role="option"');
    assert.match(chatJs, /e\.key === 'ArrowDown'/, 'ArrowDown navigation supported');
    assert.match(chatJs, /e\.key === 'ArrowUp'/, 'ArrowUp navigation supported');
  });

  test('File preview uses renderMarkdown to sanitize raw HTML', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /dom\.filePreviewContent\.innerHTML = renderMarkdown\(text\)/, 'Markdown preview uses sanitizing renderer');
    assert.match(chatJs, /dom\.filePreviewContent\.innerHTML = ''/, 'closeFilePreview clears content DOM');
  });

  test('Light mode styles avoid hardcoded dark background in topbar and terminal', () => {
    const chatCss = fs.readFileSync(new URL('../../ui/chat.css', import.meta.url), 'utf8');
    assert.match(chatCss, /\.topbar\s*\{[^}]*background:\s*var\(--panel\)/, 'Topbar uses var(--panel)');
    assert.match(chatCss, /\.terminal-drawer\s*\{[^}]*background:\s*var\(--panel\)/, 'Terminal drawer uses var(--panel)');
  });

  test('Modal focus trap and universal Escape key dismiss open dialogs', () => {
    const chatJs = fs.readFileSync(new URL('../../ui/chat.js', import.meta.url), 'utf8');
    assert.match(chatJs, /activeModal\s*&&\s*e\.key === 'Tab'/, 'Tab focus trap on active modal');
    assert.match(chatJs, /e\.key === 'Escape'/, 'Escape handler present');
    assert.match(chatJs, /closeFilePreview\(\)/, 'Escape closes preview modal');
    assert.match(chatJs, /closeDiffViewer\(\)/, 'Escape closes diff viewer');
  });
});

/* ------------------------------------------------------------------ */
/* VAL-WIN-003: UI Connection Status Indicator & User-Friendly Recovery*/
/* ------------------------------------------------------------------ */

describe('VAL-WIN-003: UI Connection Status Indicator and Non-blocking Retry', () => {
  test('VAL-WIN-003: connect_error updates connection status to reconnecting without throwing modal error alerts', () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket, dom } = harness.exports;
    const pill = harness.getEl('connection-pill');

    connectSocket();
    assert.ok(harness.listeners.connect_error, 'connect_error listener must be registered');

    // Simulate connection error on startup / disconnect
    harness.listeners.connect_error(new Error('WebSocket connection to ws:// failed'));

    // Status pill must update to reconnecting
    assert.equal(pill.dataset.state, 'reconnecting', 'pill dataset state must be reconnecting on connect_error');
    assert.match(pill.textContent, /Reconnecting/, 'pill label must show Reconnecting');

    // Intrusive error alert banner must NOT be shown
    assert.equal(dom.chatError.hidden, true, 'chatError alert must remain hidden on connect_error');
    assert.equal(dom.chatError.textContent, '', 'chatError text must be empty');

    // Retry button must become visible for user recovery
    assert.equal(dom.retryBtn.hidden, false, 'retry button must be visible during reconnecting state');
  });

  test('VAL-WIN-003: reconnect_failed transitions status to offline and keeps retry button visible', () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket, dom } = harness.exports;
    const pill = harness.getEl('connection-pill');

    connectSocket();
    assert.ok(harness.listeners.reconnect_failed, 'reconnect_failed listener must be registered');

    // Trigger reconnect_failed event
    harness.listeners.reconnect_failed();

    assert.equal(pill.dataset.state, 'offline', 'status must transition to offline when reconnect fails');
    assert.match(pill.textContent, /Offline/, 'status label must show Offline');
    assert.equal(dom.retryBtn.hidden, false, 'retry button must remain visible when offline');
    assert.equal(dom.chatError.hidden, true, 'chatError modal alert must not be shown on reconnect_failed');
  });

  test('VAL-WIN-003: manual reconnect button triggers immediate reconnection attempt', () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket, dom, state } = harness.exports;
    const pill = harness.getEl('connection-pill');

    connectSocket();
    harness.listeners.connect_error(new Error('Connection dropped'));
    assert.equal(pill.dataset.state, 'reconnecting');

    let connectCalled = false;
    state.socket.connect = () => {
      connectCalled = true;
    };

    // Simulate clicking retry button
    dom.retryBtn.dispatch('click', { preventDefault: () => {} });

    assert.equal(connectCalled, true, 'clicking retry button must trigger immediate socket connect attempt');
    assert.equal(pill.dataset.state, 'reconnecting', 'status must remain in reconnecting state');
  });

  test('VAL-WIN-003: successful connect transitions status to online and hides retry button', () => {
    const harness = createHarness();
    harness.exports.init();
    const { connectSocket, dom } = harness.exports;
    const pill = harness.getEl('connection-pill');

    connectSocket();
    // Initially reconnecting with retry button visible
    harness.listeners.connect_error(new Error('Initial failure'));
    assert.equal(dom.retryBtn.hidden, false);

    // Socket successfully connects
    harness.listeners.connect();

    assert.equal(pill.dataset.state, 'online', 'pill state must be online');
    assert.match(pill.textContent, /Online/, 'pill label must show Online');
    assert.equal(dom.retryBtn.hidden, true, 'retry button must be hidden when connected online');
    assert.equal(dom.chatError.hidden, true, 'chatError must be hidden when connected online');
  });

  test('VAL-WIN-003: ui/container.js synchronizes #connection-retry-btn across state transitions and click triggers reconnectNow', async () => {
    const harness = createContainerHarness();
    const { bridge, elements, document, createElement, calls } = harness;

    const retryBtn = createElement('connection-retry-btn');
    retryBtn.hidden = true;
    elements['connection-retry-btn'] = retryBtn;

    const badge = createElement('connection-status-badge');
    elements['connection-status-badge'] = badge;

    bridge.installDiagnostics(document);

    // Offline -> retryBtn visible
    bridge.setConnectionStatus('offline');
    assert.equal(retryBtn.hidden, false, 'retry button must be visible when offline');

    // Reconnecting -> retryBtn visible
    bridge.setConnectionStatus('reconnecting');
    assert.equal(retryBtn.hidden, false, 'retry button must be visible when reconnecting');

    // Connected -> retryBtn hidden
    bridge.setConnectionStatus('connected');
    assert.equal(retryBtn.hidden, true, 'retry button must be hidden when connected');

    // Click retry button triggers reconnectNow action
    bridge.setConnectionStatus('offline');
    assert.equal(retryBtn.hidden, false);

    let reconnectEventFired = false;
    harness.context.window.addEventListener('aqvn:reconnect', () => {
      reconnectEventFired = true;
    });

    retryBtn.dispatch('click', { preventDefault: () => {} });
    assert.equal(bridge.getConnectionStatus(), 'reconnecting', 'status must transition to reconnecting on retry click');
    assert.equal(reconnectEventFired, true, 'aqvn:reconnect event must be dispatched on retry click');

    const reconnectCalls = calls.filter((c) => c.command === 'reconnect_network');
    assert.equal(reconnectCalls.length, 1, 'reconnect_network IPC command must be dispatched');
  });

  test('VAL-WIN-003: index.html defines #connection-retry-btn and chat.css styles non-blocking pill and retry button', () => {
    const html = readIndexHtml();
    assert.match(html, /id="connection-retry-btn"/, 'ui/index.html must expose #connection-retry-btn');

    const css = readChatCss();
    assert.match(css, /\.connection-retry-btn/, 'ui/chat.css must define .connection-retry-btn');
    assert.match(css, /\.conn\[data-state="reconnecting"\]/, 'ui/chat.css must style .conn[data-state="reconnecting"]');
  });
});

describe('VAL-DEBT-001: OpenSpec Change 206 Task Reconciliation & Archive', () => {
  test('VAL-DEBT-001: change206-fix-clippy-as-chunks is archived and all tasks marked complete', () => {
    const archiveDir = new URL('../../openspec/changes/archive/2026-11-07-change206-fix-clippy-as-chunks', import.meta.url);
    assert.ok(fs.existsSync(archiveDir), 'openspec/changes/archive/2026-11-07-change206-fix-clippy-as-chunks/ must exist');

    const tasksMdPath = new URL('../../openspec/changes/archive/2026-11-07-change206-fix-clippy-as-chunks/tasks.md', import.meta.url);
    assert.ok(fs.existsSync(tasksMdPath), 'tasks.md must exist in archived directory');

    const content = fs.readFileSync(tasksMdPath, 'utf8');
    assert.doesNotMatch(content, /^- \[ \] /m, 'tasks.md in change206 must have all tasks checked off ([x])');

    const activeChange206 = new URL('../../openspec/changes/change206-fix-clippy-as-chunks', import.meta.url);
    assert.ok(!fs.existsSync(activeChange206), 'change206-fix-clippy-as-chunks must not remain under openspec/changes/');
  });
});

describe('VAL-UI-COMPONENTS-001: Web Components Custom Elements Shell', () => {
  test('ui/components/ contains all required modular custom elements', () => {
    const requiredFiles = [
      'aqvn-shell.js',
      'aqvn-topbar.js',
      'aqvn-sidebar.js',
      'aqvn-chat-pane.js',
      'aqvn-canvas-pane.js',
      'aqvn-virtual-message-list.js',
      'index.js'
    ];
    requiredFiles.forEach((file) => {
      const p = new URL(`../../ui/components/${file}`, import.meta.url);
      assert.ok(fs.existsSync(p), `ui/components/${file} must exist`);
    });
  });

  test('ui/index.html declares all W3C custom elements in DOM hierarchy', () => {
    const html = readIndexHtml();
    ['aqvn-shell', 'aqvn-topbar', 'aqvn-sidebar', 'aqvn-chat-pane', 'aqvn-canvas-pane', 'aqvn-virtual-message-list'].forEach((tag) => {
      assert.match(html, new RegExp(`<${tag}[\\s>]`), `ui/index.html must declare <${tag}>`);
    });
  });

  test('ui/styles/tokens.css defines custom element display styles and OKLCH design tokens', () => {
    const p = new URL('../../ui/styles/tokens.css', import.meta.url);
    assert.ok(fs.existsSync(p), 'ui/styles/tokens.css must exist');
    const css = fs.readFileSync(p, 'utf8');
    assert.match(css, /aqvn-shell/, 'defines aqvn-shell styles');
    assert.match(css, /aqvn-topbar/, 'defines aqvn-topbar styles');
    assert.match(css, /aqvn-sidebar/, 'defines aqvn-sidebar styles');
    assert.match(css, /aqvn-chat-pane/, 'defines aqvn-chat-pane styles');
    assert.match(css, /aqvn-canvas-pane/, 'defines aqvn-canvas-pane styles');
    assert.match(css, /aqvn-virtual-message-list/, 'defines aqvn-virtual-message-list styles');
  });
});

describe('VAL-UI-SIGNALS-002: Reactive Signals State Store & Message Virtualization', () => {
  test('ui/state/signals.js exports centralized reactive signal atoms', async () => {
    const signalsPath = new URL('../../ui/state/signals.js', import.meta.url);
    assert.ok(fs.existsSync(signalsPath), 'ui/state/signals.js must exist');
    const signalsModule = await import(signalsPath.href);
    const store = signalsModule.default || signalsModule;

    assert.equal(typeof store.signal, 'function', 'store must export signal');
    assert.equal(typeof store.computed, 'function', 'store must export computed');
    assert.equal(typeof store.connectionState.value, 'string', 'connectionState signal initialized');
    assert.equal(store.isNetworkHealthy.value, true, 'isNetworkHealthy computed signal initialized');

    // Test reactivity
    store.connectionState.value = 'offline';
    assert.equal(store.isNetworkHealthy.value, false, 'computed signal reactively updates to false when offline');

    store.connectionState.value = 'connected';
    assert.equal(store.isNetworkHealthy.value, true, 'computed signal reactively updates to true when reconnected');
  });

  test('aqvn-virtual-message-list keeps DOM nodes lean under heavy conversation load', async () => {
    const virtualListPath = new URL('../../ui/components/aqvn-virtual-message-list.js', import.meta.url);
    const virtualListCode = fs.readFileSync(virtualListPath, 'utf8');
    assert.match(virtualListCode, /customElements\.define\('aqvn-virtual-message-list'/, 'defines custom element');

    // Virtual rendering keeps visible slice bounded
    assert.match(virtualListCode, /renderVirtualSlice/, 'virtual list contains renderVirtualSlice windowing');
    assert.match(virtualListCode, /_overscan/, 'virtual list supports overscan buffer');
  });
});

describe('VAL-UI-XTERM-001: xterm.js WebGL Terminal Drawer Streaming', () => {
  test('ui/vendor/xterm.min.js and xterm.css exist and export required classes', async () => {
    const xtermJsPath = new URL('../../ui/vendor/xterm.min.js', import.meta.url);
    const xtermCssPath = new URL('../../ui/vendor/xterm.css', import.meta.url);
    assert.ok(fs.existsSync(xtermJsPath), 'ui/vendor/xterm.min.js must exist');
    assert.ok(fs.existsSync(xtermCssPath), 'ui/vendor/xterm.css must exist');

    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const xtermBundle = req(xtermJsPath.pathname);

    assert.equal(typeof xtermBundle.Terminal, 'function', 'exports Terminal class');
    assert.equal(typeof (xtermBundle.FitAddon?.FitAddon || xtermBundle.FitAddon), 'function', 'exports FitAddon class');
    assert.equal(typeof (xtermBundle.WebglAddon?.WebglAddon || xtermBundle.WebglAddon), 'function', 'exports WebglAddon class');
  });

  test('ui/components/aqvn-terminal-drawer.js encapsulates xterm drawer Custom Element', () => {
    const drawerPath = new URL('../../ui/components/aqvn-terminal-drawer.js', import.meta.url);
    assert.ok(fs.existsSync(drawerPath), 'ui/components/aqvn-terminal-drawer.js must exist');
    const drawerCode = fs.readFileSync(drawerPath, 'utf8');
    assert.match(drawerCode, /customElements\.define\('aqvn-terminal-drawer'/, 'registers <aqvn-terminal-drawer>');
    assert.match(drawerCode, /FitAddon|fit\(\)/, 'supports FitAddon auto-resize');
  });

  test('ui/index.html integrates xterm assets and declares <aqvn-terminal-drawer>', () => {
    const html = readIndexHtml();
    assert.match(html, /vendor\/xterm\.css/, 'index.html links vendor/xterm.css');
    assert.match(html, /vendor\/xterm\.min\.js/, 'index.html loads vendor/xterm.min.js');
    assert.match(html, /<aqvn-terminal-drawer[\s>]/, 'index.html declares <aqvn-terminal-drawer>');
  });

  test('Terminal initializes xterm with FitAddon and WebGL renderer on Alt+T', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initXtermTerminal, fitTerminal } = harness.exports;

    let writeCalls = [];
    let loadAddonCalls = [];
    class MockTerminal {
      constructor(opts) {
        this.options = opts || {};
        this.cols = opts.cols || 80;
        this.rows = opts.rows || 24;
      }
      loadAddon(addon) { loadAddonCalls.push(addon); }
      open(el) { this.element = el; }
      write(data) { writeCalls.push(data); }
      clear() { writeCalls.length = 0; }
      reset() { writeCalls.length = 0; }
    }
    class MockFitAddon {
      fit() { this.fitted = true; }
    }
    class MockWebglAddon {
      onContextLoss(cb) { this.lossCb = cb; }
      dispose() {}
    }

    harness.context.Terminal = MockTerminal;
    harness.context.FitAddon = MockFitAddon;
    harness.context.WebglAddon = MockWebglAddon;

    const term = initXtermTerminal();
    assert.ok(term, 'xterm instance initialized');
    assert.equal(term.cols, 80);
    assert.equal(term.rows, 24);
    assert.equal(loadAddonCalls.length, 2, 'Loads both FitAddon and WebglAddon');
    assert.equal(state.xtermWebglActive, true, 'WebGL acceleration is active for 60fps rendering');

    // Test fitTerminal
    fitTerminal();
    assert.equal(state.fitAddon.fitted, true, 'fitTerminal invokes FitAddon.fit()');
  });

  test('Terminal streams ANSI PTY chunks into xterm at 60fps and maps OKLCH themes', () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initXtermTerminal, appendTerminalOutput, applyTheme, clearTerminalOutput } = harness.exports;

    let writes = [];
    class MockTerminal {
      constructor(opts) {
        this.options = opts || {};
        this.cols = 80;
        this.rows = 24;
      }
      loadAddon() {}
      open() {}
      write(chunk) { writes.push(chunk); }
      clear() { writes.length = 0; }
    }
    harness.context.Terminal = MockTerminal;
    initXtermTerminal();

    // Streaming PTY data with ANSI escape sequences
    const ptyStreamData = '\x1b[32m[OK]\x1b[0m Cargo test build passing\n\x1b[34m[INFO]\x1b[0m Listening on 55080\n';
    appendTerminalOutput(ptyStreamData);

    assert.equal(writes.length, 1, 'Writes directly to xterm instance');
    assert.match(writes[0], /\[OK\]/, 'ANSI stream preserves sequence');
    assert.match(dom.terminalOutput.textContent, /\[OK\]/, 'Accessible DOM also retains content');

    // Test OKLCH theme synchronization
    applyTheme('cyberpunk-terminal');
    assert.equal(state.term.options.theme.foreground, '#00ff9f', 'Cyberpunk neon foreground mapped');
    assert.equal(state.term.options.theme.background, '#0d1117', 'Cyberpunk background mapped');

    applyTheme('oled-black');
    assert.equal(state.term.options.theme.background, '#000000', 'OLED black background mapped');

    // Test clear output
    clearTerminalOutput();
    assert.equal(writes.length, 0, 'Clear resets xterm buffer');
  });
});

describe('VAL-UI-EDITOR-001: CodeMirror 6 Workspace Editor & Visual Diff Viewer', () => {
  test('ui/vendor/codemirror.min.js and codemirror.css exist and export required classes', async () => {
    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const cmCssPath = new URL('../../ui/vendor/codemirror.css', import.meta.url);
    assert.ok(fs.existsSync(cmJsPath), 'ui/vendor/codemirror.min.js must exist');
    assert.ok(fs.existsSync(cmCssPath), 'ui/vendor/codemirror.css must exist');

    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    assert.equal(typeof cmBundle.EditorView, 'function', 'exports EditorView class');
    assert.equal(typeof cmBundle.EditorState, 'function', 'exports EditorState class');
    assert.equal(typeof cmBundle.MergeView, 'function', 'exports MergeView class');
    assert.equal(typeof cmBundle.detectLanguage, 'function', 'exports detectLanguage function');
    assert.equal(typeof cmBundle.computeDiffChunks, 'function', 'exports computeDiffChunks function');

    // Language detection check
    assert.equal(cmBundle.detectLanguage('app.ts'), 'ts');
    assert.equal(cmBundle.detectLanguage('main.rs'), 'rs');
    assert.equal(cmBundle.detectLanguage('script.py'), 'py');
    assert.equal(cmBundle.detectLanguage('config.json'), 'json');
  });

  test('ui/components/aqvn-code-editor.js encapsulates CodeMirror 6 Custom Element', () => {
    const editorPath = new URL('../../ui/components/aqvn-code-editor.js', import.meta.url);
    assert.ok(fs.existsSync(editorPath), 'ui/components/aqvn-code-editor.js must exist');
    const editorCode = fs.readFileSync(editorPath, 'utf8');
    assert.match(editorCode, /customElements\.define\('aqvn-code-editor'/, 'registers <aqvn-code-editor>');
    assert.match(editorCode, /setValue|setDiff|revertChunk/, 'provides editor and diff controls');
  });

  test('ui/components/index.js exports AqvnCodeEditor and ui/index.html integrates CodeMirror assets', () => {
    const indexPath = new URL('../../ui/components/index.js', import.meta.url);
    const indexCode = fs.readFileSync(indexPath, 'utf8');
    assert.match(indexCode, /aqvn-code-editor\.js/, 'imports aqvn-code-editor.js');
    assert.match(indexCode, /AqvnCodeEditor/, 'exports AqvnCodeEditor in registry');

    const html = readIndexHtml();
    assert.match(html, /vendor\/codemirror\.css/, 'index.html links vendor/codemirror.css');
    assert.match(html, /vendor\/codemirror\.min\.js/, 'index.html loads vendor/codemirror.min.js');
    assert.match(html, /<aqvn-code-editor\s+id="file-code-editor"/, 'index.html declares #file-code-editor');
    assert.match(html, /<aqvn-code-editor\s+id="diff-code-editor"/, 'index.html declares #diff-code-editor');
  });

  test('Opening file in Workspace drawer loads content directly into CodeMirror 6 editor with syntax highlighting', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initCodeMirrorEditor, openFilePreview } = harness.exports;

    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    harness.context.CodeMirror = cmBundle;

    const rustSource = 'fn main() {\n    let msg = "Hello from AQVN Workspace";\n    println!("{}", msg);\n}';
    const editor = initCodeMirrorEditor(rustSource, 'src/main.rs', { readOnly: true });

    assert.ok(editor, 'CodeMirror EditorView initialized');
    assert.equal(editor.state.doc.toString(), rustSource, 'Editor loaded with file content');
    assert.equal(editor.language, 'rs', 'Syntax language mapped to Rust (rs)');
    assert.equal(editor.state.doc.lineCount, 4, 'Correct line count calculated');
    assert.equal(state.codeEditor, editor, 'Stored active editor in state.codeEditor');
    assert.equal(state.editorDirty, false, 'Editor starts non-dirty on file open');
  });

  test('Theme switching updates CodeMirror themes across OKLCH palettes', () => {
    const harness = createHarness();
    harness.exports.init();
    const { state, getCodeMirrorTheme, applyTheme, CODEMIRROR_THEMES } = harness.exports;

    assert.ok(CODEMIRROR_THEMES, 'CODEMIRROR_THEMES dictionary defined');
    assert.ok(CODEMIRROR_THEMES['aqvn-dark'], 'aqvn-dark theme palette defined');
    assert.ok(CODEMIRROR_THEMES['oled-black'], 'oled-black theme palette defined');
    assert.ok(CODEMIRROR_THEMES['cyberpunk-terminal'], 'cyberpunk-terminal theme palette defined');
    assert.ok(CODEMIRROR_THEMES['clean-studio-light'], 'clean-studio-light theme palette defined');

    assert.equal(getCodeMirrorTheme('cyberpunk-terminal').foreground, '#00ff9f', 'Cyberpunk theme has neon green foreground');
    assert.equal(getCodeMirrorTheme('oled-black').background, '#000000', 'OLED theme has pitch black background');

    applyTheme('cyberpunk-terminal');
    assert.equal(state.theme, 'cyberpunk-terminal', 'Theme set to cyberpunk-terminal');
  });

  test('Side-by-side Visual Diff Viewer displays two-column comparison and chunk revert', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initCodeMirrorDiffViewer, openDiffViewer, closeDiffViewer } = harness.exports;

    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    harness.context.CodeMirror = cmBundle;
    if (harness.context.window) harness.context.window.CodeMirror = cmBundle;

    const origCode = 'line 1\nold line\nline 3';
    const modCode = 'line 1\nnew line\nline 3';

    const diffViewer = initCodeMirrorDiffViewer('Visual Patch', undefined, origCode, modCode);
    assert.ok(diffViewer, 'MergeView initialized');
    assert.ok(diffViewer.a, 'Original column editor exists');
    assert.ok(diffViewer.b, 'Modified column editor exists');
    assert.equal(diffViewer.a.state.doc.toString(), origCode, 'Original doc matches');
    assert.equal(diffViewer.b.state.doc.toString(), modCode, 'Modified doc matches');
    assert.equal(diffViewer.chunks.length, 1, 'Detected 1 diff chunk');
    assert.equal(diffViewer.chunks[0].type, 'modify', 'Chunk type is modify');

    // Test revert chunk
    const reverted = diffViewer.revertChunk(0);
    assert.equal(reverted, true, 'revertChunk succeeded');
    assert.equal(diffViewer.b.state.doc.toString(), origCode, 'Modified doc restored to original code');

    // Test openDiffViewer and closeDiffViewer
    openDiffViewer('Visual Diff Review', '--- a\n+++ b\n-hello\n+world');
    assert.equal(dom.diffViewerModal.hidden, false, 'diffViewerModal is open');
    closeDiffViewer();
    assert.equal(dom.diffViewerModal.hidden, true, 'diffViewerModal is closed');
    assert.equal(state.codeDiffViewer, null, 'codeDiffViewer cleared on close');
  });

  test('Saving file editor safely writes new content to Workspace and clears dirty state', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initCodeMirrorEditor, saveFileEditor, toggleFileEditor, closeFilePreview } = harness.exports;

    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    harness.context.CodeMirror = cmBundle;
    if (harness.context.window) harness.context.window.CodeMirror = cmBundle;

    state.previewingFile = { path: 'src/config.json', name: 'config.json', content: '{"port": 55080}' };
    const editor = initCodeMirrorEditor(state.previewingFile.content, state.previewingFile.path);

    // Simulate user editing the code in CodeMirror
    const updatedContent = '{"port": 55080, "host": "127.0.0.1"}';
    editor.dispatch({ changes: updatedContent });
    assert.equal(state.editorDirty, true, 'state.editorDirty flagged on document change');

    let writtenPayload = null;
    harness.context.fetch = async (url, opts) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_write')) {
        writtenPayload = JSON.parse(opts.body);
        return new Response(JSON.stringify({ ok: true, success: true }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_read')) {
        return new Response(JSON.stringify({ ok: true, content: updatedContent }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_list') || u.includes('/api/workspace_files')) {
        return new Response(JSON.stringify({ ok: true, entries: [] }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    // Save
    await saveFileEditor();
    assert.ok(writtenPayload, 'apiCall files_write triggered');
    assert.equal(writtenPayload.path, 'src/config.json', 'Writes to correct path');
    assert.equal(writtenPayload.content, updatedContent, 'Writes updated CodeMirror text');
    assert.equal(state.editorDirty, false, 'Dirty state cleared after save');
    assert.equal(state.previewingFile.content, updatedContent, 'Previewing file cache updated');

    // Close preview resets editor state
    const closed = closeFilePreview();
    assert.equal(closed, true, 'File preview closed cleanly');
    assert.equal(state.codeEditor, null, 'codeEditor instance destroyed on close');
  });

  test('openFilePreview loads text file directly into CodeMirror 6 and handles toggleFileEditor edit mode', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, openFilePreview, toggleFileEditor, closeFilePreview } = harness.exports;

    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    harness.context.CodeMirror = cmBundle;
    if (harness.context.window) harness.context.window.CodeMirror = cmBundle;

    const pythonCode = 'def calculate_metrics():\n    return {"fcp": 32.5, "unit": "ms"}\n';
    harness.context.fetch = async (url) => {
      const u = String(url);
      if (u.includes('/proxy/csrf')) {
        return new Response(JSON.stringify({ ok: true, token: 'fake-csrf' }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      if (u.includes('/api/files_read')) {
        return new Response(JSON.stringify({ ok: true, content: pythonCode }), {
          status: 200,
          headers: { 'content-type': 'application/json' }
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { 'content-type': 'application/json' }
      });
    };

    await openFilePreview({ name: 'metrics.py', size: 120 });
    assert.equal(dom.filePreviewModal.hidden, false, 'Modal opened');
    assert.ok(state.codeEditor, 'CodeMirror editor instance created');
    assert.equal(state.codeEditor.state.doc.toString(), pythonCode, 'Content loaded into CodeMirror');
    assert.equal(state.codeEditor.language, 'py', 'Language detected as Python');
    assert.equal(state.codeEditor.state.readOnly, true, 'Starts in read-only preview mode');
    assert.equal(dom.fileCodeEditor.hidden, false, 'file-code-editor is visible');
    assert.equal(dom.filePreviewContent.hidden, true, 'pre-formatted fallback is hidden when CodeMirror active');

    // Toggle into Edit mode
    toggleFileEditor();
    assert.equal(state.codeEditor.state.readOnly, false, 'Switched to editable state');
    assert.equal(dom.filePreviewEditBtn.textContent, 'Cancel', 'Edit button shows Cancel');
    assert.equal(dom.filePreviewSaveBtn.hidden, false, 'Save button is displayed');

    // Toggle back (Cancel)
    toggleFileEditor();
    assert.equal(state.codeEditor.state.readOnly, true, 'Reverted to read-only state');
    assert.equal(dom.filePreviewEditBtn.textContent, 'Edit', 'Edit button shows Edit');
    assert.equal(dom.filePreviewSaveBtn.hidden, true, 'Save button is hidden');

    closeFilePreview();
  });

  test('Unsaved changes dirty state guard warns before discarding modifications', async () => {
    const harness = createHarness();
    harness.exports.init();
    const { dom, state, initCodeMirrorEditor, closeFilePreview } = harness.exports;

    const cmJsPath = new URL('../../ui/vendor/codemirror.min.js', import.meta.url);
    const { createRequire } = await import('node:module');
    const req = createRequire(import.meta.url);
    const cmBundle = req(cmJsPath.pathname);

    harness.context.CodeMirror = cmBundle;
    if (harness.context.window) harness.context.window.CodeMirror = cmBundle;

    state.previewingFile = { path: 'test.js', name: 'test.js', content: 'console.log(1);' };
    const editor = initCodeMirrorEditor(state.previewingFile.content, state.previewingFile.path);

    // Make change
    editor.dispatch({ changes: 'console.log(2);' });
    assert.equal(state.editorDirty, true, 'Dirty state active');

    // User cancels confirmation
    harness.context.confirm = () => false;
    let closed = closeFilePreview();
    assert.equal(closed, false, 'Closing aborted when user rejects confirm');
    assert.ok(state.codeEditor, 'Editor remains open');

    // User accepts confirmation
    harness.context.confirm = () => true;
    closed = closeFilePreview();
    assert.equal(closed, true, 'Closing succeeds when user confirms discard');
    assert.equal(state.codeEditor, null, 'Editor cleared');
    assert.equal(state.editorDirty, false, 'Dirty state cleared');
  });
});














