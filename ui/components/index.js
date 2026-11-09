/**
 * AQVN Workspace Web Components Registry (W3C Custom Elements)
 * Exports and initializes all custom elements:
 * - <aqvn-shell>
 * - <aqvn-topbar>
 * - <aqvn-sidebar>
 * - <aqvn-chat-pane>
 * - <aqvn-canvas-pane>
 * - <aqvn-virtual-message-list>
 * - <aqvn-terminal-drawer>
 * - <aqvn-code-editor>
 */

(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define([], factory);
  } else if (typeof module === 'object' && module.exports) {
    var shell = require('./aqvn-shell.js');
    var topbar = require('./aqvn-topbar.js');
    var sidebar = require('./aqvn-sidebar.js');
    var chatPane = require('./aqvn-chat-pane.js');
    var canvasPane = require('./aqvn-canvas-pane.js');
    var virtualList = require('./aqvn-virtual-message-list.js');
    var terminalDrawer = require('./aqvn-terminal-drawer.js');
    var codeEditor = require('./aqvn-code-editor.js');
    module.exports = factory(shell, topbar, sidebar, chatPane, canvasPane, virtualList, terminalDrawer, codeEditor);
  } else {
    root.AqvnComponents = factory(
      { AqvnShell: root.AqvnShell },
      { AqvnTopbar: root.AqvnTopbar },
      { AqvnSidebar: root.AqvnSidebar },
      { AqvnChatPane: root.AqvnChatPane },
      { AqvnCanvasPane: root.AqvnCanvasPane },
      { AqvnVirtualMessageList: root.AqvnVirtualMessageList },
      { AqvnTerminalDrawer: root.AqvnTerminalDrawer },
      { AqvnCodeEditor: root.AqvnCodeEditor }
    );
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function (shell, topbar, sidebar, chatPane, canvasPane, virtualList, terminalDrawer, codeEditor) {
  'use strict';

  var components = {
    AqvnShell: shell && shell.AqvnShell,
    AqvnTopbar: topbar && topbar.AqvnTopbar,
    AqvnSidebar: sidebar && sidebar.AqvnSidebar,
    AqvnChatPane: chatPane && chatPane.AqvnChatPane,
    AqvnCanvasPane: canvasPane && canvasPane.AqvnCanvasPane,
    AqvnVirtualMessageList: virtualList && virtualList.AqvnVirtualMessageList,
    AqvnTerminalDrawer: terminalDrawer && terminalDrawer.AqvnTerminalDrawer,
    AqvnCodeEditor: codeEditor && codeEditor.AqvnCodeEditor
  };

  return components;
});
