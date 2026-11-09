/**
 * <aqvn-chat-pane> Custom Element (W3C Custom Elements v1)
 * Encapsulates the conversation toolbar, message list container,
 * search drawer, composer, and terminal drawer.
 */

(function () {
  'use strict';

  class AqvnChatPane extends HTMLElement {
    static get observedAttributes() {
      return ['streaming', 'unread-count'];
    }

    constructor() {
      super();
      this._disposers = [];
    }

    connectedCallback() {
      this.classList.add('aqvn-chat-pane-host');
      this._initSignalsSync();
    }

    disconnectedCallback() {
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'streaming') {
        const stopBtn = this.querySelector('#stop-btn');
        if (stopBtn) {
          stopBtn.disabled = newVal === null;
        }
      }
    }

    _initSignalsSync() {
      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.isStreaming && typeof sig.isStreaming.subscribe === 'function') {
          this._disposers.push(
            sig.isStreaming.subscribe((streaming) => {
              if (streaming) this.setAttribute('streaming', '');
              else this.removeAttribute('streaming');
            })
          );
        }
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-chat-pane')) {
    customElements.define('aqvn-chat-pane', AqvnChatPane);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnChatPane };
  } else if (typeof window !== 'undefined') {
    window.AqvnChatPane = AqvnChatPane;
  }
})();
