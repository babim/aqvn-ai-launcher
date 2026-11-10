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

              const renderBars = (container, history, max, fairThresh, poorThresh) => {
                if (!container || !Array.isArray(history) || !history.length) return;
                container.innerHTML = '';
                const m = max || Math.max.apply(null, [100].concat(history));
                history.forEach((val) => {
                  const bar = document.createElement('div');
                  const isPoor = val >= poorThresh;
                  const isFair = val >= fairThresh;
                  bar.className = 'sparkline-bar' + (isPoor ? ' poor' : (isFair ? ' fair' : ''));
                  const pct = Math.max(10, Math.min(100, Math.round((val / m) * 100)));
                  bar.style.height = pct + '%';
                  container.appendChild(bar);
                });
              };
              if (metrics.cpuHistory) {
                renderBars(this.querySelector('#telemetry-cpu-sparkline'), metrics.cpuHistory, 100, 60, 85);
              }
              if (metrics.ramHistory) {
                const maxRam = Math.max.apply(null, [1024].concat(metrics.ramHistory));
                renderBars(this.querySelector('#telemetry-ram-sparkline'), metrics.ramHistory, maxRam, maxRam * 0.7, maxRam * 0.85);
              }
              if (metrics.pingHistory) {
                const maxLat = Math.max.apply(null, [100].concat(metrics.pingHistory));
                renderBars(this.querySelector('#telemetry-latency-sparkline'), metrics.pingHistory, maxLat, 50, 150);
              }
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
