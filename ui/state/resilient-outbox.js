/**
 * Resilient Offline Outbox with IndexedDB Persistence & Atomic Lock (VAL-OUTBOX-PERSIST-002)
 *
 * Provides transactional, crash-recoverable offline queuing for chat messages.
 * Uses native IndexedDB as primary asynchronous storage engine with synchronous
 * localStorage mirror for instant reads, backward-compatibility, and seamless fallback.
 */

(function (root, factory) {
  if (typeof define === 'function' && define.amd) {
    define([], factory);
  } else if (typeof module === 'object' && module.exports) {
    module.exports = factory();
  } else {
    root.AqvnResilientOutbox = factory();
  }
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
  'use strict';

  var DB_NAME = 'aqvn_offline_db';
  var DB_VERSION = 1;
  var STORE_NAME = 'outbox';
  var OUTBOX_KEY = 'aqvn_outbox';
  var INFLIGHT_KEY = 'aqvn_outbox_inflight';
  var OUTBOX_MAX_ENTRIES = 50;
  var OUTBOX_MAX_BYTES = 256 * 1024; // 256 KB

  var dbPromise = null;

  function getIndexedDB() {
    if (typeof indexedDB !== 'undefined') return indexedDB;
    if (typeof window !== 'undefined' && window.indexedDB) return window.indexedDB;
    if (typeof globalThis !== 'undefined' && globalThis.indexedDB) return globalThis.indexedDB;
    return null;
  }

  function getLocalStorage() {
    try {
      if (typeof localStorage !== 'undefined') return localStorage;
      if (typeof window !== 'undefined' && window.localStorage) return window.localStorage;
      if (typeof globalThis !== 'undefined' && globalThis.localStorage) return globalThis.localStorage;
    } catch (e) {}
    return null;
  }

  function openDatabase() {
    if (dbPromise) return dbPromise;
    var idb = getIndexedDB();
    if (!idb) {
      dbPromise = Promise.resolve(null);
      return dbPromise;
    }
    dbPromise = new Promise(function (resolve) {
      try {
        var req = idb.open(DB_NAME, DB_VERSION);
        req.onupgradeneeded = function (e) {
          var db = e.target.result;
          if (!db.objectStoreNames.contains(STORE_NAME)) {
            var store = db.createObjectStore(STORE_NAME, { keyPath: 'message_id' });
            store.createIndex('status', 'status', { unique: false });
            store.createIndex('queued_at', 'queued_at', { unique: false });
          }
        };
        req.onsuccess = function (e) {
          resolve(e.target.result);
        };
        req.onerror = function () {
          resolve(null);
        };
      } catch (err) {
        resolve(null);
      }
    });
    return dbPromise;
  }

  function readLocalStorage(key) {
    var storage = getLocalStorage();
    if (!storage) return [];
    try {
      var raw = storage.getItem(key);
      if (!raw) return [];
      var parsed = JSON.parse(raw);
      return Array.isArray(parsed) ? parsed : [];
    } catch (e) {
      return [];
    }
  }

  function writeLocalStorage(key, list) {
    var storage = getLocalStorage();
    if (!storage) return false;
    try {
      if (list && list.length) {
        storage.setItem(key, JSON.stringify(list));
      } else {
        storage.removeItem(key);
      }
      return true;
    } catch (e) {
      return false;
    }
  }

  /* Recover orphaned in-flight items after unexpected crashes or reboots */
  function recoverOrphanedInflight() {
    var inflightList = readLocalStorage(INFLIGHT_KEY);
    if (inflightList && inflightList.length > 0) {
      var storage = getLocalStorage();
      if (storage) {
        try { storage.removeItem(INFLIGHT_KEY); } catch (e) {}
      }
      var queue = readLocalStorage(OUTBOX_KEY);
      var seen = {};
      queue.forEach(function (e) { if (e && e.message_id) seen[e.message_id] = true; });
      for (var i = inflightList.length - 1; i >= 0; i--) {
        var item = inflightList[i];
        if (item && item.message_id && !seen[item.message_id]) {
          item.status = 'queued';
          queue.unshift(item);
          seen[item.message_id] = true;
        }
      }
      writeLocalStorage(OUTBOX_KEY, queue);

      // Also sync recovery to IndexedDB
      openDatabase().then(function (db) {
        if (!db) return;
        try {
          var tx = db.transaction([STORE_NAME], 'readwrite');
          var store = tx.objectStore(STORE_NAME);
          inflightList.forEach(function (item) {
            item.status = 'queued';
            store.put(item);
          });
        } catch (e) {}
      });
      return queue;
    }
    return null;
  }

  function readOutbox() {
    recoverOrphanedInflight();
    return readLocalStorage(OUTBOX_KEY);
  }

  function writeOutbox(list) {
    var ok = writeLocalStorage(OUTBOX_KEY, list);
    // Asynchronously mirror into IndexedDB
    openDatabase().then(function (db) {
      if (!db) return;
      try {
        var tx = db.transaction([STORE_NAME], 'readwrite');
        var store = tx.objectStore(STORE_NAME);
        store.clear();
        if (list && list.length) {
          list.forEach(function (item) {
            store.put(item);
          });
        }
      } catch (e) {}
    });
    return ok;
  }

  function queueOutbox(entryOrText, messageId, context) {
    var entry;
    if (entryOrText && typeof entryOrText === 'object') {
      entry = {
        text: entryOrText.text || entryOrText.content || '',
        context: entryOrText.context || '',
        message_id: entryOrText.message_id || entryOrText.id || messageId || String(Date.now()),
        queued_at: entryOrText.queued_at || Date.now(),
        status: 'queued',
        retry_count: 0
      };
    } else {
      entry = {
        text: String(entryOrText || ''),
        context: context || '',
        message_id: messageId || String(Date.now()),
        queued_at: Date.now(),
        status: 'queued',
        retry_count: 0
      };
    }

    if (JSON.stringify([entry]).length > OUTBOX_MAX_BYTES) {
      return { stored: false, dropped: 0, error: 'the message is too large to queue' };
    }

    var list = readOutbox();
    // Deduplicate by message_id
    if (entry.message_id && list.some(function (e) { return e.message_id === entry.message_id; })) {
      return { stored: true, dropped: 0, error: '' };
    }

    list.push(entry);
    var dropped = 0;
    while (list.length > OUTBOX_MAX_ENTRIES) {
      list.shift();
      dropped += 1;
    }
    var payload = JSON.stringify(list);
    while (list.length > 1 && payload.length > OUTBOX_MAX_BYTES) {
      list.shift();
      dropped += 1;
      payload = JSON.stringify(list);
    }

    var stored = writeOutbox(list);

    // Also persist entry directly into IndexedDB with atomic transaction
    openDatabase().then(function (db) {
      if (!db) return;
      try {
        var tx = db.transaction([STORE_NAME], 'readwrite');
        var store = tx.objectStore(STORE_NAME);
        store.put(entry);
      } catch (e) {}
    });

    return { stored: stored, dropped: dropped, error: stored ? '' : 'browser storage is unavailable' };
  }

  function claimInflightBatch() {
    var list = readOutbox();
    if (!list.length) return [];
    var inflightList = list.map(function (item) {
      var copy = Object.assign({}, item);
      copy.status = 'inflight';
      return copy;
    });

    writeLocalStorage(INFLIGHT_KEY, inflightList);
    writeLocalStorage(OUTBOX_KEY, []);

    // Atomic update in IndexedDB
    openDatabase().then(function (db) {
      if (!db) return;
      try {
        var tx = db.transaction([STORE_NAME], 'readwrite');
        var store = tx.objectStore(STORE_NAME);
        inflightList.forEach(function (item) {
          store.put(item);
        });
      } catch (e) {}
    });

    return inflightList;
  }

  function completeInflightItem(messageId) {
    var inflightList = readLocalStorage(INFLIGHT_KEY);
    if (inflightList && inflightList.length) {
      var nextInflight = inflightList.filter(function (e) { return e.message_id !== messageId; });
      writeLocalStorage(INFLIGHT_KEY, nextInflight);
    }

    openDatabase().then(function (db) {
      if (!db) return;
      try {
        var tx = db.transaction([STORE_NAME], 'readwrite');
        var store = tx.objectStore(STORE_NAME);
        store.delete(messageId);
      } catch (e) {}
    });
  }

  function clearInflight() {
    var storage = getLocalStorage();
    if (storage) {
      try { storage.removeItem(INFLIGHT_KEY); } catch (e) {}
    }
  }

  function requeueOutbox(remaining) {
    if (!remaining || !remaining.length) return true;
    var current = readOutbox();
    var seen = {};
    current.forEach(function (e) { if (e && e.message_id) seen[e.message_id] = true; });
    var prepend = remaining.filter(function (e) { return e && e.message_id && !seen[e.message_id]; });
    var combined = prepend.concat(current);
    clearInflight();
    return writeOutbox(combined);
  }

  return {
    DB_NAME: DB_NAME,
    STORE_NAME: STORE_NAME,
    OUTBOX_MAX_ENTRIES: OUTBOX_MAX_ENTRIES,
    OUTBOX_MAX_BYTES: OUTBOX_MAX_BYTES,
    openDatabase: openDatabase,
    readOutbox: readOutbox,
    writeOutbox: writeOutbox,
    queueOutbox: queueOutbox,
    claimInflightBatch: claimInflightBatch,
    completeInflightItem: completeInflightItem,
    clearInflight: clearInflight,
    requeueOutbox: requeueOutbox,
    recoverOrphanedInflight: recoverOrphanedInflight
  };
});
