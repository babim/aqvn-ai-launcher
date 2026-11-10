/**
 * <aqvn-code-editor> Custom Element (W3C Custom Elements v1)
 * Wraps CodeMirror 6 inline editor with syntax highlighting, line numbers,
 * active line marking, and side-by-side visual diff merge viewer.
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
    dispatchEvent() { return true; }
  };

  class AqvnCodeEditor extends BaseElement {
    static get observedAttributes() {
      return ['mode', 'filename', 'language', 'readonly', 'theme', 'dirty'];
    }

    constructor() {
      super();
      this._view = null;
      this._mergeView = null;
      this._originalContent = '';
      this._isDirty = false;
      this._filename = '';
      this._language = 'js';
      this._syntaxWorker = null;
    }

    _initSyntaxWorker() {
      if (this._syntaxWorker) return this._syntaxWorker;
      var CodeMirror = (typeof window !== 'undefined' && window.CodeMirror) || (typeof globalThis !== 'undefined' && globalThis.CodeMirror);
      if (CodeMirror && typeof CodeMirror.createSyntaxWorker === 'function') {
        this._syntaxWorker = CodeMirror.createSyntaxWorker('workers/syntax-worker.js');
      } else if (typeof Worker !== 'undefined') {
        try {
          this._syntaxWorker = new Worker('workers/syntax-worker.js');
        } catch (e) {
          this._syntaxWorker = null;
        }
      }
      return this._syntaxWorker;
    }

    get syntaxWorker() {
      return this._syntaxWorker || this._initSyntaxWorker();
    }

    connectedCallback() {
      this.classList.add('aqvn-code-editor-host');
      if (!this._view && !this._mergeView) {
        var initialText = this.getAttribute('value') || this.textContent || '';
        if (initialText.trim() && !this.getAttribute('mode')) {
          this.setValue(initialText, this.getAttribute('filename'));
        }
      }
    }

    disconnectedCallback() {
      this.destroy();
    }

    attributeChangedCallback(name, oldVal, newVal) {
      if (oldVal === newVal) return;
      if (name === 'readonly') {
        var isReadOnly = newVal !== null && newVal !== 'false';
        if (this._view && this._view.dispatch) {
          this._view.dispatch({ readOnly: isReadOnly });
        }
      } else if (name === 'language' && newVal) {
        this.setLanguage(newVal);
      } else if (name === 'filename' && newVal) {
        this._filename = newVal;
        var CodeMirror = (typeof window !== 'undefined' && window.CodeMirror) || (typeof globalThis !== 'undefined' && globalThis.CodeMirror);
        if (CodeMirror && typeof CodeMirror.detectLanguage === 'function') {
          this.setLanguage(CodeMirror.detectLanguage(newVal));
        }
      } else if (name === 'theme' && newVal) {
        this.setTheme(newVal);
      }
    }

    get editorView() {
      return this._view;
    }

    set editorView(view) {
      this._view = view;
    }

    get mergeView() {
      return this._mergeView;
    }

    set mergeView(mv) {
      this._mergeView = mv;
    }

    get value() {
      if (this._view && this._view.state && this._view.state.doc) {
        return this._view.state.doc.toString();
      }
      return this._originalContent || '';
    }

    set value(newVal) {
      this.setValue(newVal, this._filename);
    }

    get isDirty() {
      return this._isDirty;
    }

    set isDirty(val) {
      var next = !!val;
      if (this._isDirty !== next) {
        this._isDirty = next;
        if (next) {
          this.setAttribute('dirty', '');
        } else {
          this.removeAttribute('dirty');
        }
        this.dispatchEvent(new CustomEvent('dirty', {
          detail: { dirty: next, filename: this._filename },
          bubbles: true
        }));
      }
    }

    setValue(content, filename) {
      this._originalContent = String(content || '');
      this._filename = filename || this._filename || '';
      this.isDirty = false;

      var CodeMirror = (typeof window !== 'undefined' && window.CodeMirror) || (typeof globalThis !== 'undefined' && globalThis.CodeMirror);
      if (!CodeMirror) {
        return;
      }

      if (this._filename && typeof CodeMirror.detectLanguage === 'function') {
        this._language = CodeMirror.detectLanguage(this._filename);
      }

      if (this._mergeView) {
        try { this._mergeView.destroy(); } catch (e) {}
        this._mergeView = null;
      }

      if (this._view) {
        try { this._view.destroy(); } catch (e) {}
        this._view = null;
      }

      var self = this;
      var isReadOnly = this.hasAttribute('readonly') && this.getAttribute('readonly') !== 'false';

      this._view = new CodeMirror.EditorView({
        state: CodeMirror.EditorState.create({
          doc: this._originalContent,
          readOnly: isReadOnly,
          extensions: [
            CodeMirror.lineNumbers(),
            CodeMirror.syntaxHighlighting(),
            CodeMirror.EditorView.updateListener.of(function (update) {
              if (update.docChanged) {
                var currentVal = update.state.doc.toString();
                self.isDirty = (currentVal !== self._originalContent);
                self.dispatchEvent(new CustomEvent('change', {
                  detail: { value: currentVal, isDirty: self.isDirty },
                  bubbles: true
                }));
              }
            })
          ]
        }),
        parent: this,
        language: this._language,
        worker: this._initSyntaxWorker()
      });
    }

    setDiff(original, modified, options) {
      options = options || {};
      this.setAttribute('mode', 'diff');

      var CodeMirror = (typeof window !== 'undefined' && window.CodeMirror) || (typeof globalThis !== 'undefined' && globalThis.CodeMirror);
      if (!CodeMirror || typeof CodeMirror.MergeView !== 'function') {
        return;
      }

      if (this._view) {
        try { this._view.destroy(); } catch (e) {}
        this._view = null;
      }

      if (this._mergeView) {
        try { this._mergeView.destroy(); } catch (e) {}
        this._mergeView = null;
      }

      var lang = options.language || (options.filename && CodeMirror.detectLanguage ? CodeMirror.detectLanguage(options.filename) : this._language);

      this._mergeView = new CodeMirror.MergeView({
        a: { doc: String(original || '') },
        b: { doc: String(modified || ''), readOnly: options.readOnly !== undefined ? !!options.readOnly : false },
        parent: this,
        language: lang
      });
    }

    revertChunk(chunkIndex) {
      if (this._mergeView && typeof this._mergeView.revertChunk === 'function') {
        return this._mergeView.revertChunk(chunkIndex);
      }
      return false;
    }

    setLanguage(lang) {
      this._language = lang;
      if (this._view && typeof this._view.setLanguage === 'function') {
        this._view.setLanguage(lang);
      }
    }

    setTheme(themeName) {
      this.setAttribute('theme', themeName);
      if (this._view && this._view.dom) {
        this._view.dom.setAttribute('data-theme', themeName);
      }
      if (this._mergeView && this._mergeView.dom) {
        this._mergeView.dom.setAttribute('data-theme', themeName);
      }
    }

    focus() {
      if (this._view && typeof this._view.focus === 'function') {
        this._view.focus();
      }
    }

    destroy() {
      if (this._syntaxWorker) {
        try { if (typeof this._syntaxWorker.terminate === 'function') this._syntaxWorker.terminate(); } catch (e) {}
        this._syntaxWorker = null;
      }
      if (this._view) {
        try { this._view.destroy(); } catch (e) {}
        this._view = null;
      }
      if (this._mergeView) {
        try { this._mergeView.destroy(); } catch (e) {}
        this._mergeView = null;
      }
    }
  }

  if (typeof customElements !== 'undefined' && !customElements.get('aqvn-code-editor')) {
    customElements.define('aqvn-code-editor', AqvnCodeEditor);
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AqvnCodeEditor };
  } else if (typeof window !== 'undefined') {
    window.AqvnCodeEditor = AqvnCodeEditor;
  }
})();
