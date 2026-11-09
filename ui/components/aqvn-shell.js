/**
 * <aqvn-shell> Custom Element (W3C Custom Elements v1)
 * Encapsulates the 3-column workspace layout, drag-resize splitters,
 * and Focus Mode (F11 / Ctrl+Shift+F).
 */

(function () {
  'use strict';

  class AqvnShell extends HTMLElement {
    static get observedAttributes() {
      return ['focus-mode', 'sidebar-collapsed', 'canvas-collapsed'];
    }

    constructor() {
      super();
      this._focusMode = false;
      this._sidebarCollapsed = false;
      this._canvasCollapsed = false;
      this._disposers = [];
    }

    connectedCallback() {
      this.classList.add('aqvn-shell-host');
      this._initSignalsSync();
    }

    disconnectedCallback() {
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'focus-mode') {
        this.focusMode = newVal !== null;
      } else if (name === 'sidebar-collapsed') {
        this.sidebarCollapsed = newVal !== null;
      } else if (name === 'canvas-collapsed') {
        this.canvasCollapsed = newVal !== null;
      }
    }

    get focusMode() {
      return this._focusMode;
    }

    set focusMode(val) {
      this._focusMode = Boolean(val);
      if (this._focusMode) {
        this.setAttribute('focus-mode', '');
        document.body.classList.add('focus-mode');
      } else {
        this.removeAttribute('focus-mode');
        document.body.classList.remove('focus-mode');
      }
    }

    get sidebarCollapsed() {
      return this._sidebarCollapsed;
    }

    set sidebarCollapsed(val) {
      this._sidebarCollapsed = Boolean(val);
      const sidebar = this.querySelector('#sidebar');
      if (sidebar) {
        if (this._sidebarCollapsed) {
          sidebar.setAttribute('hidden', '');
          sidebar.classList.add('collapsed');
        } else {
          sidebar.removeAttribute('hidden');
          sidebar.classList.remove('collapsed');
        }
      }
    }

    get canvasCollapsed() {
      return this._canvasCollapsed;
    }

    set canvasCollapsed(val) {
      this._canvasCollapsed = Boolean(val);
      const canvas = this.querySelector('#canvas-pane');
      if (canvas) {
        if (this._canvasCollapsed) {
          canvas.setAttribute('hidden', '');
        } else {
          canvas.removeAttribute('hidden');
        }
      }
    }

    _initSignalsSync() {
      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.sidebarCollapsed && typeof sig.sidebarCollapsed.subscribe === 'function') {
          this._disposers.push(
            sig.sidebarCollapsed.subscribe((collapsed) => {
              this.sidebarCollapsed = collapsed;
            })
          );
        }
        if (sig.canvasCollapsed && typeof sig.canvasCollapsed.subscribe === 'function') {
          this._disposers.push(
            sig.canvasCollapsed.subscribe((collapsed) => {
              this.canvasCollapsed = collapsed;
            })
          );
        }
        if (sig.focusMode && typeof sig.focusMode.subscribe === 'function') {
          this._disposers.push(
            sig.focusMode.subscribe((focused) => {
              this.focusMode = focused;
            })
          );
        }
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-shell')) {
    customElements.define('aqvn-shell', AqvnShell);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnShell };
  } else if (typeof window !== 'undefined') {
    window.AqvnShell = AqvnShell;
  }
})();
