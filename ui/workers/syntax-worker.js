/**
 * Dedicated Syntax Highlighting Web Worker
 * Offloads CodeMirror 6 tokenization and AST parsing from the main UI thread.
 * License: MIT
 */

(function () {
  'use strict';

  var KEYWORDS = {
    js: new Set([
      'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger', 'default',
      'delete', 'do', 'else', 'export', 'extends', 'finally', 'for', 'function',
      'if', 'import', 'in', 'instanceof', 'new', 'return', 'super', 'switch',
      'this', 'throw', 'try', 'typeof', 'var', 'void', 'while', 'with', 'yield',
      'let', 'static', 'async', 'await', 'of', 'from', 'as'
    ]),
    ts: new Set([
      'break', 'case', 'catch', 'class', 'const', 'continue', 'debugger', 'default',
      'delete', 'do', 'else', 'export', 'extends', 'finally', 'for', 'function',
      'if', 'import', 'in', 'instanceof', 'new', 'return', 'super', 'switch',
      'this', 'throw', 'try', 'typeof', 'var', 'void', 'while', 'with', 'yield',
      'let', 'static', 'async', 'await', 'of', 'from', 'as', 'type', 'interface',
      'namespace', 'declare', 'enum', 'implements', 'private', 'protected', 'public',
      'readonly', 'any', 'unknown', 'never', 'keyof'
    ]),
    rs: new Set([
      'as', 'break', 'const', 'continue', 'crate', 'else', 'enum', 'extern',
      'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
      'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct',
      'super', 'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while',
      'async', 'await', 'dyn'
    ]),
    py: new Set([
      'False', 'None', 'True', 'and', 'as', 'assert', 'async', 'await', 'break',
      'class', 'continue', 'def', 'del', 'elif', 'else', 'except', 'finally',
      'for', 'from', 'global', 'if', 'import', 'in', 'is', 'lambda', 'nonlocal',
      'not', 'or', 'pass', 'raise', 'return', 'try', 'while', 'with', 'yield'
    ]),
    sh: new Set([
      'if', 'then', 'else', 'elif', 'fi', 'case', 'esac', 'for', 'while', 'until',
      'do', 'done', 'in', 'function', 'select', 'return', 'exit', 'echo', 'export',
      'local', 'readonly', 'set', 'unset'
    ]),
    json: new Set(['true', 'false', 'null']),
    c: new Set([
      'auto', 'break', 'case', 'char', 'const', 'continue', 'default', 'do',
      'double', 'else', 'enum', 'extern', 'float', 'for', 'goto', 'if', 'int',
      'long', 'register', 'return', 'short', 'signed', 'sizeof', 'static',
      'struct', 'switch', 'typedef', 'union', 'unsigned', 'void', 'volatile', 'while'
    ]),
    cpp: new Set([
      'class', 'public', 'private', 'protected', 'virtual', 'template', 'typename',
      'namespace', 'using', 'new', 'delete', 'this', 'friend', 'inline', 'constexpr',
      'nullptr', 'auto', 'bool', 'catch', 'throw', 'try'
    ]),
    go: new Set([
      'break', 'case', 'chan', 'const', 'continue', 'default', 'defer', 'else',
      'fallthrough', 'for', 'func', 'go', 'goto', 'if', 'import', 'interface',
      'map', 'package', 'range', 'return', 'select', 'struct', 'switch', 'type', 'var'
    ])
  };

  function detectLanguage(filename) {
    if (!filename) return 'js';
    var ext = String(filename).split('.').pop().toLowerCase();
    switch (ext) {
      case 'ts':
      case 'tsx':
        return 'ts';
      case 'rs':
        return 'rs';
      case 'py':
        return 'py';
      case 'sh':
      case 'bash':
      case 'zsh':
        return 'sh';
      case 'json':
        return 'json';
      case 'html':
      case 'xml':
      case 'svg':
        return 'html';
      case 'css':
      case 'scss':
        return 'css';
      case 'md':
      case 'markdown':
        return 'md';
      case 'yaml':
      case 'yml':
        return 'yaml';
      case 'c':
      case 'h':
        return 'c';
      case 'cpp':
      case 'cc':
      case 'cxx':
      case 'hpp':
        return 'cpp';
      case 'go':
        return 'go';
      default:
        return 'js';
    }
  }

  function escapeHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function tokenizeLine(lineText, lang) {
    var words = KEYWORDS[lang] || KEYWORDS.js;
    var tokens = [];
    var out = '';
    var i = 0;
    var len = lineText ? lineText.length : 0;

    if (len === 0) {
      return { html: '&nbsp;', tokens: tokens };
    }

    while (i < len) {
      var ch = lineText[i];

      // Single line comments
      if (ch === '/' && lineText[i + 1] === '/') {
        var comment = lineText.slice(i);
        out += '<span class="tok-comment">' + escapeHtml(comment) + '</span>';
        tokens.push({ type: 'comment', start: i, end: len, text: comment });
        break;
      }
      if (ch === '#' && (lang === 'py' || lang === 'sh' || lang === 'yaml')) {
        var pyComment = lineText.slice(i);
        out += '<span class="tok-comment">' + escapeHtml(pyComment) + '</span>';
        tokens.push({ type: 'comment', start: i, end: len, text: pyComment });
        break;
      }

      // Strings
      if (ch === '"' || ch === "'" || ch === '`') {
        var quote = ch;
        var end = i + 1;
        while (end < len && lineText[end] !== quote) {
          if (lineText[end] === '\\') end++;
          end++;
        }
        if (end < len) end++;
        var strVal = lineText.slice(i, end);
        out += '<span class="tok-string">' + escapeHtml(strVal) + '</span>';
        tokens.push({ type: 'string', start: i, end: end, text: strVal });
        i = end;
        continue;
      }

      // Numbers
      if (/\d/.test(ch) && (i === 0 || /[\s\(\[\{=+\-*\/,]/.test(lineText[i - 1]))) {
        var numEnd = i;
        while (numEnd < len && /[\d.xX_a-fA-F]/.test(lineText[numEnd])) {
          numEnd++;
        }
        var numVal = lineText.slice(i, numEnd);
        out += '<span class="tok-number">' + escapeHtml(numVal) + '</span>';
        tokens.push({ type: 'number', start: i, end: numEnd, text: numVal });
        i = numEnd;
        continue;
      }

      // Identifiers / Keywords / Functions / Types
      if (/[a-zA-Z_$]/.test(ch)) {
        var wordEnd = i;
        while (wordEnd < len && /[a-zA-Z0-9_$]/.test(lineText[wordEnd])) {
          wordEnd++;
        }
        var word = lineText.slice(i, wordEnd);
        if (words && words.has(word)) {
          out += '<span class="tok-keyword">' + escapeHtml(word) + '</span>';
          tokens.push({ type: 'keyword', start: i, end: wordEnd, text: word });
        } else if (lineText[wordEnd] === '(') {
          out += '<span class="tok-function">' + escapeHtml(word) + '</span>';
          tokens.push({ type: 'function', start: i, end: wordEnd, text: word });
        } else if (/^[A-Z][a-zA-Z0-9_]*$/.test(word)) {
          out += '<span class="tok-type">' + escapeHtml(word) + '</span>';
          tokens.push({ type: 'type', start: i, end: wordEnd, text: word });
        } else {
          out += '<span class="tok-variable">' + escapeHtml(word) + '</span>';
          tokens.push({ type: 'variable', start: i, end: wordEnd, text: word });
        }
        i = wordEnd;
        continue;
      }

      // Operators / punctuation
      if (/[=+\-*\/%&|^!~?:<>]/.test(ch)) {
        out += '<span class="tok-operator">' + escapeHtml(ch) + '</span>';
        tokens.push({ type: 'operator', start: i, end: i + 1, text: ch });
        i++;
        continue;
      }

      out += escapeHtml(ch);
      i++;
    }

    return { html: out, tokens: tokens };
  }

  function highlightLine(lineText, lang) {
    return tokenizeLine(lineText, lang).html;
  }

  function highlightBatch(lines, lang) {
    if (!Array.isArray(lines)) return { lines: [], tokens: [] };
    var highlighted = [];
    var allTokens = [];
    for (var i = 0; i < lines.length; i++) {
      var res = tokenizeLine(lines[i], lang);
      highlighted.push(res.html);
      allTokens.push(res.tokens);
    }
    return { lines: highlighted, tokens: allTokens };
  }

  function handleWorkerMessage(data) {
    if (!data) return null;
    var id = data.id || null;
    var lang = data.language || (data.filename ? detectLanguage(data.filename) : 'js');
    var rawLines = null;

    if (Array.isArray(data.lines)) {
      rawLines = data.lines;
    } else if (typeof data.text === 'string') {
      rawLines = data.text.split('\n');
    } else {
      rawLines = [''];
    }

    var result = highlightBatch(rawLines, lang);
    return {
      id: id,
      type: 'highlight_result',
      language: lang,
      lineCount: rawLines.length,
      lines: result.lines,
      tokens: result.tokens,
      success: true
    };
  }

  // Bind to Web Worker listener if in Web Worker environment
  if (typeof self !== 'undefined' && typeof self.postMessage === 'function') {
    self.addEventListener('message', function (event) {
      var response = handleWorkerMessage(event.data);
      if (response) {
        self.postMessage(response);
      }
    });
  }

  // Export for Node.js test suites
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
      handleWorkerMessage: handleWorkerMessage,
      highlightLine: highlightLine,
      tokenizeLine: tokenizeLine,
      highlightBatch: highlightBatch,
      detectLanguage: detectLanguage,
      KEYWORDS: KEYWORDS
    };
  } else if (typeof window !== 'undefined') {
    window.AqvnSyntaxWorker = {
      handleWorkerMessage: handleWorkerMessage,
      highlightLine: highlightLine,
      tokenizeLine: tokenizeLine,
      highlightBatch: highlightBatch,
      detectLanguage: detectLanguage,
      KEYWORDS: KEYWORDS
    };
  }
})();
