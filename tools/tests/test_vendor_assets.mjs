// Contract checks for VAL-MD-001: vendor marked/highlight assets staged in
// ui/vendor/ and loaded by ui/index.html before chat.js.
import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const uiDir = path.join(repoRoot, 'ui');
const vendorDir = path.join(uiDir, 'vendor');

function readVendor(name) {
  return fs.readFileSync(path.join(vendorDir, name), 'utf8');
}

function evalInBrowserScope(source) {
  const sandbox = { window: null, document: undefined };
  sandbox.window = sandbox;
  vm.createContext(sandbox);
  vm.runInContext(source, sandbox, { filename: 'vendor.js' });
  return sandbox;
}

describe('ui/vendor assets (VAL-MD-001)', () => {
  test('marked.min.js is present and valid JS', () => {
    const file = path.join(vendorDir, 'marked.min.js');
    assert.ok(fs.existsSync(file), 'ui/vendor/marked.min.js must exist');
    const src = readVendor('marked.min.js');
    assert.ok(src.length > 1000, 'marked.min.js must not be a stub');
    assert.doesNotThrow(() => new vm.Script(src), 'marked.min.js must parse as JS');
  });

  test('highlight.min.js is present and valid JS', () => {
    const file = path.join(vendorDir, 'highlight.min.js');
    assert.ok(fs.existsSync(file), 'ui/vendor/highlight.min.js must exist');
    const src = readVendor('highlight.min.js');
    assert.ok(src.length > 1000, 'highlight.min.js must not be a stub');
    assert.doesNotThrow(() => new vm.Script(src), 'highlight.min.js must parse as JS');
  });

  test('window.marked and window.hljs are defined in the webview window scope', () => {
    const scope = evalInBrowserScope(readVendor('marked.min.js') + '\n' + readVendor('highlight.min.js'));
    assert.ok(scope.marked, 'window.marked must be defined');
    assert.equal(typeof scope.marked.parse, 'function', 'marked.parse must be callable');
    assert.ok(scope.hljs, 'window.hljs must be defined');
    assert.equal(typeof scope.hljs.highlight, 'function', 'hljs.highlight must be callable');
  });

  test('ui/index.html loads marked.min.js and highlight.min.js before chat.js', () => {
    const html = fs.readFileSync(path.join(uiDir, 'index.html'), 'utf8');
    const markedAt = html.indexOf('vendor/marked.min.js');
    const highlightAt = html.indexOf('vendor/highlight.min.js');
    const chatAt = html.indexOf('src="chat.js"');
    assert.ok(markedAt > -1, 'index.html must reference vendor/marked.min.js');
    assert.ok(highlightAt > -1, 'index.html must reference vendor/highlight.min.js');
    assert.ok(chatAt > -1, 'index.html must reference chat.js');
    assert.ok(markedAt < chatAt, 'marked.min.js must load before chat.js');
    assert.ok(highlightAt < chatAt, 'highlight.min.js must load before chat.js');
  });
});
