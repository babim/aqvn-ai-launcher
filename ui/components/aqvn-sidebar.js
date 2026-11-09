/**
 * <aqvn-sidebar> Custom Element (W3C Custom Elements v1)
 * Encapsulates sessions navigation, tab switching (Sessions / Workspace),
 * workspace file tree, and breadcrumbs.
 */

(function () {
  'use strict';

  class AqvnSidebar extends HTMLElement {
    static get observedAttributes() {
      return ['active-tab', 'collapsed'];
    }

    constructor() {
      super();
      this._disposers = [];
    }

    connectedCallback() {
      this.classList.add('aqvn-sidebar-host');
      this._initSignalsSync();
    }

    disconnectedCallback() {
      this._disposers.forEach(function (d) { try { d(); } catch (e) {} });
      this._disposers = [];
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'active-tab' && newVal) {
        this.switchTab(newVal);
      }
    }

    switchTab(tabName) {
      const sessionsBtn = this.querySelector('#tab-sessions-btn');
      const workspaceBtn = this.querySelector('#tab-workspace-btn');
      const sessionsView = this.querySelector('#sessions-view');
      const workspaceView = this.querySelector('#workspace-view');

      if (tabName === 'workspace') {
        if (sessionsBtn) {
          sessionsBtn.classList.remove('active');
          sessionsBtn.setAttribute('aria-selected', 'false');
        }
        if (workspaceBtn) {
          workspaceBtn.classList.add('active');
          workspaceBtn.setAttribute('aria-selected', 'true');
        }
        if (sessionsView) sessionsView.setAttribute('hidden', '');
        if (workspaceView) workspaceView.removeAttribute('hidden');
      } else {
        if (sessionsBtn) {
          sessionsBtn.classList.add('active');
          sessionsBtn.setAttribute('aria-selected', 'true');
        }
        if (workspaceBtn) {
          workspaceBtn.classList.remove('active');
          workspaceBtn.setAttribute('aria-selected', 'false');
        }
        if (sessionsView) sessionsView.removeAttribute('hidden');
        if (workspaceView) workspaceView.setAttribute('hidden', '');
      }

      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.activeSidebarTab && sig.activeSidebarTab.value !== tabName) {
          sig.activeSidebarTab.value = tabName;
        }
      }
    }

    _initSignalsSync() {
      if (typeof window !== 'undefined' && window.AqvnSignals) {
        const sig = window.AqvnSignals;
        if (sig.activeSidebarTab && typeof sig.activeSidebarTab.subscribe === 'function') {
          this._disposers.push(
            sig.activeSidebarTab.subscribe((tab) => {
              this.setAttribute('active-tab', tab);
            })
          );
        }
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-sidebar')) {
    customElements.define('aqvn-sidebar', AqvnSidebar);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnSidebar };
  } else if (typeof window !== 'undefined') {
    window.AqvnSidebar = AqvnSidebar;
  }
})();
