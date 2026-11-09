/**
 * <aqvn-canvas-pane> Custom Element (W3C Custom Elements v1)
 * Encapsulates the Intel Suite canvas drawer and embedded iframe.
 */

(function () {
  'use strict';

  class AqvnCanvasPane extends HTMLElement {
    static get observedAttributes() {
      return ['collapsed'];
    }

    constructor() {
      super();
      this._disposers = [];
    }

    connectedCallback() {
      this.classList.add('aqvn-canvas-pane-host');
      this._initSignalsSync();
    }

    disconnectedCallback() {
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'collapsed') {
        const pane = this.querySelector('#canvas-pane') || this;
        if (newVal !== null) {
          pane.setAttribute('hidden', '');
        } else {
          pane.removeAttribute('hidden');
        }
      }
    }

    _initSignalsSync() {
      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.canvasCollapsed && typeof sig.canvasCollapsed.subscribe === 'function') {
          this._disposers.push(
            sig.canvasCollapsed.subscribe((collapsed) => {
              if (collapsed) this.setAttribute('collapsed', '');
              else this.removeAttribute('collapsed');
            })
          );
        }
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-canvas-pane')) {
    customElements.define('aqvn-canvas-pane', AqvnCanvasPane);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnCanvasPane };
  } else if (typeof window !== 'undefined') {
    window.AqvnCanvasPane = AqvnCanvasPane;
  }
})();
