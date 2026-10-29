/**
 * Settings UI controller for AQVN Workspace.
 * Handles loading, validating, editing, and persisting configuration
 * via the bridge store interface (Tauri store or localStorage fallback).
 */

(function () {
  'use strict';

  const DEFAULTS = {
    server_url: 'https://a0.matmagoc.com',
    server_profiles: ['https://a0.matmagoc.com'],
    rfc_enabled: false,
    rfc_password: '',
    ssh_password: '',
    allowed_paths: ['~/Workspace'],
    rfc_port: 55080,
    ssh_port: 55022,
    connector_enabled: true,
    scope_files: true,
    scope_exec: true,
    scope_browser: true,
    scope_desktop: true,
    global_hotkey: 'Alt+Space'
  };

  let store = null;
  let allowedPaths = [...DEFAULTS.allowed_paths];
  let serverProfiles = [...DEFAULTS.server_profiles];

  // DOM Elements
  const serverUrlInput = document.getElementById('server-url');
  const serverProfilesList = document.getElementById('server-profiles-list');
  const newProfileInput = document.getElementById('new-profile-input');
  const addProfileBtn = document.getElementById('add-profile-btn');
  const connectorEnabledInput = document.getElementById('connector-enabled');
  const scopeFilesInput = document.getElementById('scope-files');
  const scopeExecInput = document.getElementById('scope-exec');
  const scopeBrowserInput = document.getElementById('scope-browser');
  const scopeDesktopInput = document.getElementById('scope-desktop');
  const rfcEnabledInput = document.getElementById('rfc-enabled');
  const rfcPasswordInput = document.getElementById('rfc-password');
  const sshPasswordInput = document.getElementById('ssh-password');
  const rfcPortInput = document.getElementById('rfc-port');
  const sshPortInput = document.getElementById('ssh-port');
  const allowedPathsList = document.getElementById('allowed-paths-list');
  const allowedPathInput = document.getElementById('allowed-path-input');
  const addAllowedPathBtn = document.getElementById('add-allowed-path');
  const globalHotkeyInput = document.getElementById('global-hotkey');
  const resetHotkeyBtn = document.getElementById('reset-hotkey-btn');
  const settingsForm = document.getElementById('settings-form');
  const saveBtn = document.getElementById('save-settings');
  const resetBtn = document.getElementById('reset-settings');
  const saveStatus = document.getElementById('save-status');

  function renderServerProfiles() {
    if (!serverProfilesList) return;
    serverProfilesList.innerHTML = '';
    const activeUrl = serverUrlInput.value.trim();

    serverProfiles.forEach((url, index) => {
      const li = document.createElement('li');
      li.className = 'path-item';

      const span = document.createElement('span');
      span.textContent = url;
      li.appendChild(span);

      const btnGroup = document.createElement('div');
      btnGroup.style.display = 'flex';
      btnGroup.style.alignItems = 'center';
      btnGroup.style.gap = '8px';

      if (url === activeUrl) {
        const badge = document.createElement('span');
        badge.className = 'active-badge';
        badge.textContent = 'Active';
        btnGroup.appendChild(badge);
      } else {
        const selectBtn = document.createElement('button');
        selectBtn.type = 'button';
        selectBtn.className = 'select-profile-btn';
        selectBtn.textContent = 'Select';
        selectBtn.setAttribute('aria-label', `Select server profile ${url}`);
        selectBtn.addEventListener('click', () => {
          serverUrlInput.value = url;
          renderServerProfiles();
        });
        btnGroup.appendChild(selectBtn);
      }

      const removeBtn = document.createElement('button');
      removeBtn.type = 'button';
      removeBtn.className = 'remove-path-btn';
      removeBtn.textContent = 'Remove';
      removeBtn.setAttribute('aria-label', `Remove server profile ${url}`);
      removeBtn.addEventListener('click', () => {
        serverProfiles.splice(index, 1);
        if (serverProfiles.length === 0) {
          serverProfiles.push(DEFAULTS.server_url);
        }
        if (serverUrlInput.value === url) {
          serverUrlInput.value = serverProfiles[0] || DEFAULTS.server_url;
        }
        renderServerProfiles();
      });
      btnGroup.appendChild(removeBtn);

      li.appendChild(btnGroup);
      serverProfilesList.appendChild(li);
    });
  }

  function addProfile() {
    if (!newProfileInput) return;
    const val = newProfileInput.value.trim();
    if (!val) return;
    if (!val.startsWith('http://') && !val.startsWith('https://')) {
      showStatus('Please enter a valid server URL starting with http:// or https://', 'error');
      newProfileInput.focus();
      return;
    }
    if (!serverProfiles.includes(val)) {
      serverProfiles.push(val);
      renderServerProfiles();
    }
    newProfileInput.value = '';
    newProfileInput.focus();
  }

  function renderAllowedPaths() {
    allowedPathsList.innerHTML = '';
    allowedPaths.forEach((path, index) => {
      const li = document.createElement('li');
      li.className = 'path-item';

      const span = document.createElement('span');
      span.textContent = path;
      li.appendChild(span);

      const removeBtn = document.createElement('button');
      removeBtn.type = 'button';
      removeBtn.className = 'remove-path-btn';
      removeBtn.textContent = 'Remove';
      removeBtn.setAttribute('aria-label', `Remove path ${path}`);
      removeBtn.addEventListener('click', () => {
        allowedPaths.splice(index, 1);
        renderAllowedPaths();
      });
      li.appendChild(removeBtn);

      allowedPathsList.appendChild(li);
    });
  }

  function addPath() {
    const val = allowedPathInput.value.trim();
    if (!val) return;
    if (!allowedPaths.includes(val)) {
      allowedPaths.push(val);
      renderAllowedPaths();
    }
    allowedPathInput.value = '';
    allowedPathInput.focus();
  }

  function showStatus(message, type = 'success') {
    saveStatus.textContent = message;
    saveStatus.className = `status-message ${type}`;
    saveStatus.style.display = 'block';
    setTimeout(() => {
      if (type === 'success') {
        saveStatus.style.display = 'none';
      }
    }, 4000);
  }

  async function loadSettings() {
    try {
      store = await window.bridge.getStore('settings.json');

      const serverUrl = await store.get('server_url');
      serverUrlInput.value = serverUrl !== undefined ? serverUrl : DEFAULTS.server_url;

      const storedProfiles = await store.get('server_profiles');
      if (Array.isArray(storedProfiles) && storedProfiles.length > 0) {
        serverProfiles = [...storedProfiles];
      } else if (window.bridge && typeof window.bridge.getServerProfiles === 'function') {
        serverProfiles = await window.bridge.getServerProfiles();
      } else {
        serverProfiles = [...DEFAULTS.server_profiles];
      }
      if (!serverProfiles.includes(serverUrlInput.value)) {
        serverProfiles.unshift(serverUrlInput.value);
      }
      renderServerProfiles();

      const rfcEnabled = await store.get('rfc_enabled');
      rfcEnabledInput.checked = rfcEnabled !== undefined ? Boolean(rfcEnabled) : DEFAULTS.rfc_enabled;

      const rfcPassword = await store.get('rfc_password');
      rfcPasswordInput.value = rfcPassword !== undefined ? rfcPassword : DEFAULTS.rfc_password;

      const sshPassword = await store.get('ssh_password');
      sshPasswordInput.value = sshPassword !== undefined ? sshPassword : DEFAULTS.ssh_password;

      const storedPaths = await store.get('allowed_paths');
      if (Array.isArray(storedPaths)) {
        allowedPaths = [...storedPaths];
      } else {
        allowedPaths = [...DEFAULTS.allowed_paths];
      }
      renderAllowedPaths();

      const rfcPort = await store.get('rfc_port');
      rfcPortInput.value = rfcPort !== undefined ? rfcPort : DEFAULTS.rfc_port;

      const sshPort = await store.get('ssh_port');
      sshPortInput.value = sshPort !== undefined ? sshPort : DEFAULTS.ssh_port;

      const connectorEnabled = await store.get('connector_enabled');
      if (connectorEnabledInput) {
        connectorEnabledInput.checked =
          connectorEnabled !== undefined ? Boolean(connectorEnabled) : DEFAULTS.connector_enabled;
      }

      const scopeFiles = await store.get('scope_files');
      if (scopeFilesInput) {
        scopeFilesInput.checked =
          scopeFiles !== undefined ? Boolean(scopeFiles) : DEFAULTS.scope_files;
      }

      const scopeExec = await store.get('scope_exec');
      if (scopeExecInput) {
        scopeExecInput.checked =
          scopeExec !== undefined ? Boolean(scopeExec) : DEFAULTS.scope_exec;
      }

      const scopeBrowser = await store.get('scope_browser');
      if (scopeBrowserInput) {
        scopeBrowserInput.checked =
          scopeBrowser !== undefined ? Boolean(scopeBrowser) : DEFAULTS.scope_browser;
      }

      const scopeDesktop = await store.get('scope_desktop');
      if (scopeDesktopInput) {
        scopeDesktopInput.checked =
          scopeDesktop !== undefined ? Boolean(scopeDesktop) : DEFAULTS.scope_desktop;
      }

      const globalHotkey = await store.get('global_hotkey');
      if (globalHotkeyInput) {
        globalHotkeyInput.value =
          globalHotkey !== undefined && globalHotkey ? globalHotkey : DEFAULTS.global_hotkey;
      }
    } catch (err) {
      console.error('Failed to load settings from store:', err);
      // Fall back to defaults
      serverUrlInput.value = DEFAULTS.server_url;
      serverProfiles = [...DEFAULTS.server_profiles];
      renderServerProfiles();
      rfcEnabledInput.checked = DEFAULTS.rfc_enabled;
      rfcPasswordInput.value = DEFAULTS.rfc_password;
      sshPasswordInput.value = DEFAULTS.ssh_password;
      allowedPaths = [...DEFAULTS.allowed_paths];
      renderAllowedPaths();
      rfcPortInput.value = DEFAULTS.rfc_port;
      sshPortInput.value = DEFAULTS.ssh_port;
      if (connectorEnabledInput) connectorEnabledInput.checked = DEFAULTS.connector_enabled;
      if (scopeFilesInput) scopeFilesInput.checked = DEFAULTS.scope_files;
      if (scopeExecInput) scopeExecInput.checked = DEFAULTS.scope_exec;
      if (scopeBrowserInput) scopeBrowserInput.checked = DEFAULTS.scope_browser;
      if (scopeDesktopInput) scopeDesktopInput.checked = DEFAULTS.scope_desktop;
      if (globalHotkeyInput) globalHotkeyInput.value = DEFAULTS.global_hotkey;
    }
  }

  async function saveSettings(e) {
    if (e) {
      if (typeof e.preventDefault === 'function') e.preventDefault();
      if (typeof e.stopPropagation === 'function') e.stopPropagation();
    }

    const serverUrl = serverUrlInput.value.trim();
    if (!serverUrl || (!serverUrl.startsWith('http://') && !serverUrl.startsWith('https://'))) {
      showStatus('Please enter a valid server URL starting with http:// or https://', 'error');
      serverUrlInput.focus();
      return;
    }

    const rfcPort = parseInt(rfcPortInput.value, 10);
    if (isNaN(rfcPort) || rfcPort < 1 || rfcPort > 65535) {
      showStatus('RFC port must be between 1 and 65535.', 'error');
      rfcPortInput.focus();
      return;
    }

    const sshPort = parseInt(sshPortInput.value, 10);
    if (isNaN(sshPort) || sshPort < 1 || sshPort > 65535) {
      showStatus('SSH port must be between 1 and 65535.', 'error');
      sshPortInput.focus();
      return;
    }

    const rfcEnabled = rfcEnabledInput.checked;
    const rfcPassword = rfcPasswordInput.value;
    const sshPassword = sshPasswordInput.value;

    if (!serverProfiles.includes(serverUrl)) {
      serverProfiles.push(serverUrl);
    }

    try {
      if (!store) {
        store = await window.bridge.getStore('settings.json');
      }

      await store.set('server_url', serverUrl);
      await store.set('server_profiles', serverProfiles);
      await store.set('rfc_enabled', rfcEnabled);
      await store.set('rfc_password', rfcPassword);
      await store.set('ssh_password', sshPassword);
      await store.set('allowed_paths', allowedPaths);
      await store.set('rfc_port', rfcPort);
      await store.set('ssh_port', sshPort);

      const connectorEnabled = connectorEnabledInput
        ? connectorEnabledInput.checked
        : DEFAULTS.connector_enabled;
      const scopeFiles = scopeFilesInput ? scopeFilesInput.checked : DEFAULTS.scope_files;
      const scopeExec = scopeExecInput ? scopeExecInput.checked : DEFAULTS.scope_exec;
      const scopeBrowser = scopeBrowserInput ? scopeBrowserInput.checked : DEFAULTS.scope_browser;
      const scopeDesktop = scopeDesktopInput ? scopeDesktopInput.checked : DEFAULTS.scope_desktop;

      await store.set('connector_enabled', connectorEnabled);
      await store.set('scope_files', scopeFiles);
      await store.set('scope_exec', scopeExec);
      await store.set('scope_browser', scopeBrowser);
      await store.set('scope_desktop', scopeDesktop);

      const globalHotkey = (
        (globalHotkeyInput ? globalHotkeyInput.value : DEFAULTS.global_hotkey) || DEFAULTS.global_hotkey
      ).trim();
      await store.set('global_hotkey', globalHotkey || DEFAULTS.global_hotkey);

      await store.save();

      if (window.bridge && typeof window.bridge.saveServerProfiles === 'function') {
        await window.bridge.saveServerProfiles(serverProfiles);
      }
      try {
        localStorage.setItem('aqvn_server_url', serverUrl);
        localStorage.setItem('aqvn_server_profiles', JSON.stringify(serverProfiles));
        localStorage.setItem('aqvn_global_hotkey', globalHotkey || DEFAULTS.global_hotkey);
      } catch (e) {}

      renderServerProfiles();
      showStatus('Settings saved successfully. [backend: ' + window.bridge.backendDescription() + ']', 'success');
    } catch (err) {
      console.error('Failed to save settings:', err);
      showStatus('Failed to save settings: ' + (err.message || String(err)), 'error');
    }
  }

  function resetDefaults() {
    serverUrlInput.value = DEFAULTS.server_url;
    serverProfiles = [...DEFAULTS.server_profiles];
    renderServerProfiles();
    rfcEnabledInput.checked = DEFAULTS.rfc_enabled;
    rfcPasswordInput.value = DEFAULTS.rfc_password;
    sshPasswordInput.value = DEFAULTS.ssh_password;
    allowedPaths = [...DEFAULTS.allowed_paths];
    renderAllowedPaths();
    rfcPortInput.value = DEFAULTS.rfc_port;
    sshPortInput.value = DEFAULTS.ssh_port;
    if (connectorEnabledInput) connectorEnabledInput.checked = DEFAULTS.connector_enabled;
    if (scopeFilesInput) scopeFilesInput.checked = DEFAULTS.scope_files;
    if (scopeExecInput) scopeExecInput.checked = DEFAULTS.scope_exec;
    if (scopeBrowserInput) scopeBrowserInput.checked = DEFAULTS.scope_browser;
    if (scopeDesktopInput) scopeDesktopInput.checked = DEFAULTS.scope_desktop;
    if (globalHotkeyInput) globalHotkeyInput.value = DEFAULTS.global_hotkey;
    showStatus('Reset to defaults (click Save to persist).', 'success');
  }

  // Event Listeners
  if (addProfileBtn) {
    addProfileBtn.addEventListener('click', addProfile);
  }
  if (newProfileInput) {
    newProfileInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        addProfile();
      }
    });
  }
  if (serverUrlInput) {
    serverUrlInput.addEventListener('input', renderServerProfiles);
  }

  addAllowedPathBtn.addEventListener('click', addPath);
  allowedPathInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      addPath();
    }
  });

  if (resetHotkeyBtn && globalHotkeyInput) {
    resetHotkeyBtn.addEventListener('click', () => {
      globalHotkeyInput.value = DEFAULTS.global_hotkey;
    });
  }

  settingsForm.addEventListener('submit', saveSettings);
  saveBtn.addEventListener('click', saveSettings);
  resetBtn.addEventListener('click', resetDefaults);

  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
      e.preventDefault();
      saveSettings();
    }
  });

  // Initial load
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', loadSettings);
  } else {
    loadSettings();
  }
})();
