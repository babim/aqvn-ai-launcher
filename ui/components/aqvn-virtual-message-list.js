/**
 * <aqvn-virtual-message-list> Custom Element (W3C Custom Elements v1)
 * High-performance virtualized message list with True Viewport Culling.
 * Detaches / unmounts <article> message nodes outside the viewport,
 * strictly keeping mounted DOM nodes < 60 nodes regardless of conversation length.
 * License: MIT
 */

(function () {
  'use strict';

  var BaseElement = (typeof HTMLElement !== 'undefined') ? HTMLElement : class {
    constructor() {
      this.classList = {
        add: function () {},
        remove: function () {},
        contains: function () { return false; }
      };
      this._attrs = {};
    }
    setAttribute(k, v) { this._attrs[k] = String(v); }
    getAttribute(k) { return this._attrs[k] !== undefined ? this._attrs[k] : null; }
    hasAttribute(k) { return this._attrs[k] !== undefined; }
    removeAttribute(k) { delete this._attrs[k]; }
    querySelector() { return null; }
    querySelectorAll() { return []; }
  };

  class AqvnVirtualMessageList extends BaseElement {
    static get observedAttributes() {
      return ['item-count', 'virtualized', 'force-culling'];
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

      // True Viewport Culling state
      this._nodePool = new Map();    // id or key -> article element
      this._allNodes = [];           // tracked message nodes in chronological order
      this._maxDomNodes = 50;        // maximum mounted DOM nodes (always strictly < 60)
      this._visibleRange = { startIndex: 0, endIndex: 0, count: 0 };
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

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'virtualized') {
        this._isVirtualized = newVal !== 'false';
        this.renderVirtualSlice();
      } else if (name === 'force-culling') {
        this.renderVirtualSlice();
      }
    }

    _setupVirtualViewport() {
      if (this.querySelector('.virtual-spacer-top')) {
        this._spacerTop = this.querySelector('.virtual-spacer-top');
        this._spacerBottom = this.querySelector('.virtual-spacer-bottom');
        return;
      }

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
        if (this._isVirtualized && (this.getTotalCount() > 35 || this.hasAttribute('force-culling'))) {
          this._scheduleRender();
        }
      };
      this._container.addEventListener('scroll', this._scrollListener, { passive: true });
    }

    _scheduleRender() {
      if (this._renderRaf) cancelAnimationFrame(this._renderRaf);
      if (typeof requestAnimationFrame === 'function') {
        this._renderRaf = requestAnimationFrame(() => this.renderVirtualSlice());
      } else {
        this.renderVirtualSlice();
      }
    }

    _syncKnownNodes() {
      if (!this._container) return;
      // Scan container for <article> elements that were added directly
      var articles = this._container.querySelectorAll ? this._container.querySelectorAll('article') : [];
      for (var i = 0; i < articles.length; i++) {
        var art = articles[i];
        var id = (art.dataset && art.dataset.id) || art.getAttribute('data-id') || ('__auto_' + this._allNodes.length);
        if (!this._nodePool.has(id)) {
          this._nodePool.set(id, art);
          if (this._allNodes.indexOf(art) === -1) {
            this._allNodes.push(art);
          }
        }
      }
    }

    _findStartIndex(scrollTop) {
      if (this._heightCache.size === 0) {
        return Math.max(0, Math.floor(scrollTop / this._itemHeight) - this._overscan);
      }
      var total = this.getTotalCount();
      var low = 0, high = total - 1;
      while (low <= high) {
        var mid = (low + high) >> 1;
        var offset = mid * this._itemHeight;
        if (offset < scrollTop) {
          low = mid + 1;
        } else {
          high = mid - 1;
        }
      }
      return Math.max(0, low - this._overscan);
    }

    trackNode(node, id) {
      if (!node) return;
      var key = id || (node.dataset && node.dataset.id) || ('__tracked_' + this._allNodes.length);
      this._nodePool.set(key, node);
      if (this._allNodes.indexOf(node) === -1) {
        this._allNodes.push(node);
      }
      if (this._isVirtualized && (this.getTotalCount() > 35 || this.hasAttribute('force-culling'))) {
        this.renderVirtualSlice();
      }
    }

    registerArticle(node, id) {
      this.trackNode(node, id);
    }

    setMessages(list) {
      this._messages = Array.isArray(list) ? list.slice() : [];
      this.setAttribute('item-count', String(this._messages.length));
      if (this._messages.length === 0) {
        this._allNodes = [];
        this._nodePool.clear();
        if (this._spacerTop) this._spacerTop.style.height = '0px';
        if (this._spacerBottom) this._spacerBottom.style.height = '0px';
        this._visibleRange = { startIndex: 0, endIndex: 0, count: 0 };
        return;
      }
      if (this._isVirtualized && (this.getTotalCount() > 35 || this.hasAttribute('force-culling'))) {
        this.renderVirtualSlice();
      }
    }

    appendMessage(msg) {
      this._messages.push(msg);
      this.setAttribute('item-count', String(this._messages.length));
      if (this._isVirtualized && (this.getTotalCount() > 35 || this.hasAttribute('force-culling'))) {
        this.renderVirtualSlice();
      }
    }

    renderVirtualSlice() {
      if (!this._container) {
        return;
      }

      this._syncKnownNodes();

      const total = this.getTotalCount();
      if (total <= 35 && !this.hasAttribute('force-culling')) {
        if (this._spacerTop) this._spacerTop.style.height = '0px';
        if (this._spacerBottom) this._spacerBottom.style.height = '0px';
        this._visibleRange = { startIndex: 0, endIndex: total, count: total };
        return;
      }

      const scrollTop = (this._container.scrollTop !== undefined) ? this._container.scrollTop : this._scrollTop;
      this._scrollTop = scrollTop;
      const clientHeight = (this._container.clientHeight !== undefined && this._container.clientHeight > 0)
        ? this._container.clientHeight
        : this._viewportHeight;

      const rawStart = this._findStartIndex(scrollTop);
      var rawEnd = Math.min(
        total,
        Math.ceil((scrollTop + clientHeight) / this._itemHeight) + this._overscan
      );

      // True Viewport Culling: Enforce maximum mounted DOM window (strictly < 60 nodes)
      if (rawEnd - rawStart > this._maxDomNodes) {
        rawEnd = Math.min(total, rawStart + this._maxDomNodes);
      }

      const startIndex = Math.max(0, rawStart);
      const endIndex = Math.min(total, rawEnd);

      const topOffset = startIndex * this._itemHeight;
      const bottomOffset = Math.max(0, (total - endIndex) * this._itemHeight);

      if (this._spacerTop) this._spacerTop.style.height = `${topOffset}px`;
      if (this._spacerBottom) this._spacerBottom.style.height = `${bottomOffset}px`;

      this._visibleRange = { startIndex, endIndex, count: endIndex - startIndex };

      // True Viewport DOM Culling: Unmount/detach <article> elements outside visible slice
      if (this._allNodes.length > 0) {
        // Phase 1: Detach all nodes outside [startIndex, endIndex)
        for (var i = 0; i < this._allNodes.length; i++) {
          if (i < startIndex || i >= endIndex) {
            var outNode = this._allNodes[i];
            if (outNode && outNode.parentNode === this._container) {
              this._container.removeChild(outNode);
            }
          }
        }

        // Phase 2: Mount and sequence in-range nodes before _spacerBottom
        var anchor = this._spacerBottom || null;
        for (var j = startIndex; j < endIndex; j++) {
          var inNode = this._allNodes[j];
          if (!inNode) continue;
          if (inNode.parentNode !== this._container) {
            if (anchor && anchor.parentNode === this._container) {
              this._container.insertBefore(inNode, anchor);
            } else {
              this._container.appendChild(inNode);
            }
          }
        }
      }
    }

    cullViewport() {
      this.renderVirtualSlice();
      return this.getMountedNodeCount();
    }

    setScrollTop(val) {
      this._scrollTop = Number(val) || 0;
      if (this._container) {
        this._container.scrollTop = this._scrollTop;
      }
      this.renderVirtualSlice();
    }

    getMountedNodeCount() {
      if (!this._container) return 0;
      if (this._container.querySelectorAll) {
        return this._container.querySelectorAll('article').length;
      }
      if (this._container.getElementsByTagName) {
        return this._container.getElementsByTagName('article').length;
      }
      return 0;
    }

    getVisibleCount() {
      if (this.getTotalCount() <= 35 && !this.hasAttribute('force-culling')) {
        return this.getTotalCount();
      }
      return this._visibleRange ? this._visibleRange.count : Math.min(35, this.getTotalCount());
    }

    getTotalCount() {
      return Math.max(this._messages.length, this._allNodes.length);
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
