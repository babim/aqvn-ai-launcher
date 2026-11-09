/**
 * <aqvn-topbar> Custom Element (W3C Custom Elements v1)
 * Encapsulates the application topbar, server switcher, telemetry indicators,
 * theme toggle, and connection status badge.
 */

(function () {
  'use strict';

  class AqvnTopbar extends HTMLElement {
    static get observedAttributes() {
      return ['theme', 'connection-state'];
    }

    constructor() {
      super();
      this._disposers = [];
    }

    connectedCallback() {
      this.classList.add('aqvn-topbar-host');
      this._initSignalsSync();
    }

    disconnectedCallback() {
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'theme' && newVal) {
        document.documentElement.setAttribute('data-theme', newVal);
      }
    }

    _initSignalsSync() {
      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.connectionState && typeof sig.connectionState.subscribe === 'function') {
          this._disposers.push(
            sig.connectionState.subscribe((st) => {
              this.setAttribute('connection-state', st);
              const badge = this.querySelector('#connection-status-badge');
              if (badge) {
                badge.setAttribute('data-state', st);
              }
            })
          );
        }
        if (sig.currentTheme && typeof sig.currentTheme.subscribe === 'function') {
          this._disposers.push(
            sig.currentTheme.subscribe((theme) => {
              this.setAttribute('theme', theme);
            })
          );
        }
        if (sig.hostMetrics && typeof sig.hostMetrics.subscribe === 'function') {
          this._disposers.push(
            sig.hostMetrics.subscribe((metrics) => {
              if (!metrics) return;
              const cpuVal = this.querySelector('#telemetry-cpu-val');
              const ramVal = this.querySelector('#telemetry-ram-val');
              const latVal = this.querySelector('#telemetry-latency-val');
              if (cpuVal && metrics.cpu !== undefined) cpuVal.textContent = metrics.cpu + '%';
              if (ramVal && metrics.ramMb !== undefined) ramVal.textContent = metrics.ramMb + ' MB';
              if (latVal && metrics.latencyMs !== undefined) latVal.textContent = metrics.latencyMs + ' ms';
            })
          );
        }
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-topbar')) {
    customElements.define('aqvn-topbar', AqvnTopbar);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnTopbar };
  } else if (typeof window !== 'undefined') {
    window.AqvnTopbar = AqvnTopbar;
  }
})();
