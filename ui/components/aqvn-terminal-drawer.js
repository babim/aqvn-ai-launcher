/**
 * <aqvn-terminal-drawer> Custom Element (W3C Custom Elements v1)
 * Wraps interactive xterm.js WebGL Terminal Drawer with FitAddon auto-resize,
 * 60fps WebGL GPU rendering, and ANSI PTY streaming.
 */

(function () {
  'use strict';

  class AqvnTerminalDrawer extends HTMLElement {
    static get observedAttributes() {
      return ['open', 'status', 'active-tab'];
    }

    constructor() {
      super();
      this._disposers = [];
      this._term = null;
      this._fitAddon = null;
      this._webglAddon = null;
      this._resizeObserver = null;
    }

    connectedCallback() {
      this.classList.add('aqvn-terminal-drawer-host');
      this._setupResizeObserver();
    }

    disconnectedCallback() {
      if (this._resizeObserver) {
        this._resizeObserver.disconnect();
        this._resizeObserver = null;
      }
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'open') {
        const drawer = this.querySelector('#terminal-drawer') || this;
        if (newVal !== null) {
          drawer.removeAttribute('hidden');
          this.fit();
        } else {
          drawer.setAttribute('hidden', '');
        }
      } else if (name === 'status') {
        const statusEl = this.querySelector('#terminal-status');
        if (statusEl && newVal) {
          statusEl.textContent = newVal;
        }
      }
    }

    get terminal() {
      return this._term;
    }

    set terminal(instance) {
      this._term = instance;
    }

    get fitAddon() {
      return this._fitAddon;
    }

    set fitAddon(addon) {
      this._fitAddon = addon;
    }

    get webglAddon() {
      return this._webglAddon;
    }

    set webglAddon(addon) {
      this._webglAddon = addon;
    }

    fit() {
      if (this._fitAddon && typeof this._fitAddon.fit === 'function') {
        try {
          this._fitAddon.fit();
        } catch (e) {}
      }
    }

    _setupResizeObserver() {
      if (typeof ResizeObserver === 'function') {
        this._resizeObserver = new ResizeObserver(() => {
          this.fit();
        });
        this._resizeObserver.observe(this);
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-terminal-drawer')) {
    customElements.define('aqvn-terminal-drawer', AqvnTerminalDrawer);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnTerminalDrawer };
  } else if (typeof window !== 'undefined') {
    window.AqvnTerminalDrawer = AqvnTerminalDrawer;
  }
})();
