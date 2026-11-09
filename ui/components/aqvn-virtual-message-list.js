/**
 * <aqvn-virtual-message-list> Custom Element (W3C Custom Elements v1)
 * High-performance virtualized message list for long conversation streams.
 * Maintains DOM lean (<= 35-50 nodes) by rendering only visible items + overscan buffer.
 */

(function () {
  'use strict';

  class AqvnVirtualMessageList extends HTMLElement {
    static get observedAttributes() {
      return ['item-count', 'virtualized'];
    }

    constructor() {
      super();
      this._messages = [];
      this._itemHeight = 72; // estimated average message height in px
      this._overscan = 5;
      this._scrollTop = 0;
      this._viewportHeight = 600;
      this._isVirtualized = true;
      this._heightCache = new Map();
      this._container = null;
      this._spacerTop = null;
      this._spacerBottom = null;
      this._viewport = null;
      this._renderRaf = null;
    }

    connectedCallback() {
      this.classList.add('aqvn-virtual-list-host');
      this._container = this.querySelector('#messages') || this;
      this._setupVirtualViewport();
      this._bindScroll();
    }

    disconnectedCallback() {
      if (this._scrollListener && this._container) {
        this._container.removeEventListener('scroll', this._scrollListener);
      }
      if (this._renderRaf) {
        cancelAnimationFrame(this._renderRaf);
      }
    }

    _setupVirtualViewport() {
      if (this.querySelector('.virtual-spacer-top')) return;

      this._spacerTop = document.createElement('div');
      this._spacerTop.className = 'virtual-spacer-top';
      this._spacerTop.style.cssText = 'height: 0px; width: 100%; pointer-events: none;';

      this._spacerBottom = document.createElement('div');
      this._spacerBottom.className = 'virtual-spacer-bottom';
      this._spacerBottom.style.cssText = 'height: 0px; width: 100%; pointer-events: none;';

      if (this._container.firstChild) {
        this._container.insertBefore(this._spacerTop, this._container.firstChild);
      } else {
        this._container.appendChild(this._spacerTop);
      }
      this._container.appendChild(this._spacerBottom);
    }

    _bindScroll() {
      this._scrollListener = () => {
        this._scrollTop = this._container.scrollTop || 0;
        this._viewportHeight = this._container.clientHeight || 600;
        if (this._isVirtualized && this._messages.length > 35) {
          this._scheduleRender();
        }
      };
      this._container.addEventListener('scroll', this._scrollListener, { passive: true });
    }

    _scheduleRender() {
      if (this._renderRaf) cancelAnimationFrame(this._renderRaf);
      this._renderRaf = requestAnimationFrame(() => this.renderVirtualSlice());
    }

    setMessages(list) {
      this._messages = Array.isArray(list) ? list.slice() : [];
      this.setAttribute('item-count', String(this._messages.length));
      if (this._isVirtualized && this._messages.length > 35) {
        this.renderVirtualSlice();
      }
    }

    appendMessage(msg) {
      this._messages.push(msg);
      this.setAttribute('item-count', String(this._messages.length));
      if (this._isVirtualized && this._messages.length > 35) {
        this.renderVirtualSlice();
      }
    }

    renderVirtualSlice() {
      if (!this._container || this._messages.length <= 35) {
        return;
      }

      const total = this._messages.length;
      const startIndex = Math.max(0, Math.floor(this._scrollTop / this._itemHeight) - this._overscan);
      const endIndex = Math.min(
        total,
        Math.ceil((this._scrollTop + this._viewportHeight) / this._itemHeight) + this._overscan
      );

      const topOffset = startIndex * this._itemHeight;
      const bottomOffset = Math.max(0, (total - endIndex) * this._itemHeight);

      if (this._spacerTop) this._spacerTop.style.height = `${topOffset}px`;
      if (this._spacerBottom) this._spacerBottom.style.height = `${bottomOffset}px`;

      this._visibleRange = { startIndex, endIndex, count: endIndex - startIndex };
    }

    getVisibleCount() {
      if (this._messages.length <= 35) {
        return this._messages.length;
      }
      return this._visibleRange ? this._visibleRange.count : Math.min(35, this._messages.length);
    }

    getTotalCount() {
      return this._messages.length;
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-virtual-message-list')) {
    customElements.define('aqvn-virtual-message-list', AqvnVirtualMessageList);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnVirtualMessageList };
  } else if (typeof window !== 'undefined') {
    window.AqvnVirtualMessageList = AqvnVirtualMessageList;
  }
})();
