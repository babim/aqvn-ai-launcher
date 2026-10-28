import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

function createHarness() {
  const filePath = new URL('../../ui/chat.js', import.meta.url);
  let code = fs.readFileSync(filePath, 'utf8');

  // Hook to extract functions before the closing IIFE
  const hook = `
__harness__.exports = {
  isLoginPage,
  apiCall,
  refreshCsrf,
  connectSocket,
  disconnectSocket,
  requestState,
  startPolling,
  stopPolling,
  onServerRestart,
  sessionExpired,
  state,
  dom,
  showChatError
};
`;
  const lastIndex = code.lastIndexOf('})();');
  if (lastIndex !== -1) {
    code = code.slice(0, lastIndex) + hook + '\n})();';
  }

  const elements = {};
  function getEl(id) {
    if (!elements[id]) {
      elements[id] = {
        id,
        value: '',
        textContent: '',
        hidden: false,
        disabled: false,
        dataset: {},
        focus: () => {},
        addEventListener: () => {},
        appendChild: () => {},
        options: []
      };
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

  const context = {
    __harness__: {},
    console,
    Intl,
    crypto,
    fetch: null,
    Headers,
    Response,
    URLSearchParams,
    document: {
      title: '',
      readyState: 'loading',
      body: {
        classList: {
          toggle: () => {}
        }
      },
      getElementById: getEl,
      createElement: () => ({ appendChild: () => {}, value: '', textContent: '', dataset: {} }),
      createTextNode: () => ({}),
      addEventListener: () => {}
    },
    window: {
      location: { origin: 'http://localhost:3000', search: '' },
      localStorage: {
        getItem: () => null,
        setItem: () => {}
      },
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
    getEl
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
