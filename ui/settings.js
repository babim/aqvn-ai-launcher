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
    workspace_path: '~/Workspace',
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
    global_hotkey: 'Alt+Space',
    close_to_tray: true,
    show_menu_bar: false,
    app_language: 'vi'
  };

  let store = null;
  let allowedPaths = [...DEFAULTS.allowed_paths];
  let serverProfiles = [...DEFAULTS.server_profiles];
  let currentLang = 'vi';

  // DOM Elements
  const appLanguageSelect = document.getElementById('app-language');
  const closeToTrayInput = document.getElementById('close-to-tray');
  const showMenuBarInput = document.getElementById('show-menu-bar');
  const langToggleBtn = document.getElementById('lang-toggle-btn');
  const currentLangText = document.getElementById('current-lang-text');
  const serverUrlInput = document.getElementById('server-url');
  const serverProfilesList = document.getElementById('server-profiles-list');
  const newProfileInput = document.getElementById('new-profile-input');
  const addProfileBtn = document.getElementById('add-profile-btn');
  const connectorEnabledInput = document.getElementById('connector-enabled');
  const scopeFilesInput = document.getElementById('scope-files');
  const scopeExecInput = document.getElementById('scope-exec');
  const scopeBrowserInput = document.getElementById('scope-browser');
  const scopeDesktopInput = document.getElementById('scope-desktop');
  const workspacePathInput = document.getElementById('workspace-path');
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

  const SETTINGS_I18N = {
    vi: {
      page_title: 'AQVN Workspace — Cài đặt',
      header_title: 'Cài đặt',
      header_subtitle: 'Cấu hình ứng dụng AQVN Workspace, máy chủ kết nối và quyền điều khiển agent.',
      back_to_chat: '← Quay lại chat',
      general_title: 'Cấu hình chung',
      language_label: 'Ngôn ngữ hiển thị (Display Language)',
      language_help: 'Chọn ngôn ngữ hiển thị cho giao diện cài đặt và tài liệu.',
      close_to_tray_label: 'Chạy ngầm ở khay hệ thống (Close to Tray)',
      close_to_tray_help: 'Tiếp tục chạy ngầm trong khay hệ thống khi đóng cửa sổ thay vì tắt hoàn toàn ứng dụng.',
      show_menu_bar_label: 'Hiển thị thanh menu (Show Menu Bar)',
      show_menu_bar_help: 'Hiển thị thanh menu native dưới thanh tiêu đề cửa sổ (áp dụng ngay; chuột phải context menu luôn khả dụng).',
      server_title: 'Cấu hình máy chủ',
      server_url_label: 'Địa chỉ máy chủ (Server URL)',
      server_url_help: 'Địa chỉ máy chủ làm việc từ xa (upstream workspace server).',
      saved_profiles_label: 'Hồ sơ máy chủ đã lưu',
      saved_profiles_help: 'Danh sách địa chỉ máy chủ. Nhấn "Chọn" để kích hoạt.',
      add_profile_btn: 'Thêm hồ sơ',
      gateway_title: 'Cổng ra Gateway Hub (_a0_connector)',
      connector_toggle: 'Bật cổng kết nối ra ngoài (Outbound Gateway)',
      connector_help: 'Công tắc tổng kết nối ra ngoài với máy chủ qua WebSocket. Tắt đi sẽ ngắt kết nối ngay lập tức.',
      security_scopes_label: 'Phạm vi bảo mật (Security Scopes)',
      security_scopes_help: 'Quyền bảo mật chi tiết khai báo trong bắt tay connector_hello.',
      scope_files_title: 'Remote Files',
      scope_files_desc: ' — đọc, ghi và sửa tệp trong thư mục cho phép',
      scope_exec_title: 'Code Execution',
      scope_exec_desc: ' — chạy lệnh shell trong phiên PTY cục bộ',
      scope_browser_title: 'Host Browser',
      scope_browser_desc: ' — tự động hóa Chrome/Edge qua CDP',
      scope_desktop_title: 'Computer Use',
      scope_desktop_desc: ' — chụp màn hình và điều khiển chuột/phím',
      workspace_path_label: 'Thư mục không gian làm việc (workspace_path)',
      workspace_path_help: 'Thư mục gốc được cô lập (sandbox) cho các thao tác tệp từ xa và đồng bộ cây thư mục (mặc định: ~/Workspace).',
      agent_control_title: 'Điều khiển Agent (RFC & SSH)',
      rfc_toggle: 'Cho phép điều khiển Agent',
      security_warning: 'Cảnh báo bảo mật: Bật tính năng này cho phép Agent từ xa truy cập tệp và chạy lệnh qua cầu nối RFC và SSH. Chỉ bật khi bạn tin cậy máy chủ và mạng.',
      rfc_password_label: 'Mật khẩu RFC',
      ssh_password_label: 'Mật khẩu SSH',
      rfc_port_label: 'Cổng RFC',
      ssh_port_label: 'Cổng SSH',
      allowed_paths_label: 'Đường dẫn cho phép',
      allowed_paths_help: 'Các thư mục trên máy mà agent được phép truy cập.',
      add_path_btn: 'Thêm đường dẫn',
      shortcuts_title: 'Phím tắt bàn phím',
      global_hotkey_label: 'Phím tắt gọi nhanh toàn cục',
      reset_hotkey_btn: 'Đặt lại mặc định',
      global_hotkey_help: 'Phím tắt toàn cục để ẩn/hiện cửa sổ từ bất kỳ đâu (ví dụ: Alt+Space, Ctrl+Alt+A). Trên Linux X11 dùng XGrabKey; trên Wayland dùng biểu tượng khay hệ thống.',
      view_help_link: 'Xem toàn bộ phím tắt & Hướng dẫn (F1) →',
      save_settings_btn: 'Lưu cài đặt',
      reset_defaults_btn: 'Đặt lại mặc định',
      save_success: 'Đã lưu cấu hình thành công.',
      save_error: 'Lưu cấu hình thất bại: ',
      reset_status: 'Đã đặt lại về mặc định (nhấn Lưu để áp dụng).'
    },
    en: {
      page_title: 'AQVN Workspace — Settings',
      header_title: 'Settings',
      header_subtitle: 'Configure AQVN Workspace client, upstream server, and agent control.',
      back_to_chat: '← Back to chat',
      general_title: 'General Configuration',
      language_label: 'Display Language',
      language_help: 'Select interface language for settings and help documentation.',
      close_to_tray_label: 'Close to Tray (Keep running in background)',
      close_to_tray_help: 'Keep the app running in the system tray when closing the window instead of exiting completely.',
      show_menu_bar_label: 'Show Menu Bar (Top Window Menu)',
      show_menu_bar_help: 'Display the native menu bar under the title bar (hot-reloaded immediately; right-click context menu is always available).',
      server_title: 'Server Configuration',
      server_url_label: 'Server URL',
      server_url_help: 'Upstream workspace server endpoint.',
      saved_profiles_label: 'Saved Server Profiles',
      saved_profiles_help: 'Saved workspace instance endpoints. Click "Select" to make a profile active.',
      add_profile_btn: 'Add Profile',
      gateway_title: 'Outbound Gateway Hub (_a0_connector)',
      connector_toggle: 'Enable Outbound Gateway',
      connector_help: 'Master switch connecting outbound to the upstream workspace server via WebSocket. Toggling off immediately terminates the outbound connection.',
      security_scopes_label: 'Security Scopes',
      security_scopes_help: 'Granular permission scopes declared in connector_hello handshake.',
      scope_files_title: 'Remote Files',
      scope_files_desc: ' — read, write, and patch files in allowed workspace paths',
      scope_exec_title: 'Code Execution',
      scope_exec_desc: ' — run shell commands inside local PTY sessions',
      scope_browser_title: 'Host Browser',
      scope_browser_desc: ' — automate local Chrome/Edge via CDP',
      scope_desktop_title: 'Computer Use',
      scope_desktop_desc: ' — screen capture and mouse/keyboard injection',
      workspace_path_label: 'Workspace Directory (workspace_path)',
      workspace_path_help: 'Root directory sandboxed for remote file operations and file tree snapshots (defaults to ~/Workspace).',
      agent_control_title: 'Agent Control (RFC & SSH)',
      rfc_toggle: 'Allow agent control',
      security_warning: 'Security Warning: Enabling agent control allows remote automated agents to access files in allowed paths and execute commands via RFC and SSH bridges. Keep this disabled unless you trust the remote agent and network environment.',
      rfc_password_label: 'RFC Password',
      ssh_password_label: 'SSH Password',
      rfc_port_label: 'RFC Port',
      ssh_port_label: 'SSH Port',
      allowed_paths_label: 'Allowed Paths',
      allowed_paths_help: 'Directories accessible to the agent.',
      add_path_btn: 'Add Path',
      shortcuts_title: 'Keyboard Shortcuts',
      global_hotkey_label: 'Global Summon Shortcut',
      reset_hotkey_btn: 'Reset Default',
      global_hotkey_help: 'Global hotkey to show/hide the window from anywhere (e.g., Alt+Space, Ctrl+Alt+A, Super+Space). On Linux X11 this uses XGrabKey; on Wayland use the system tray menu.',
      view_help_link: 'View All Shortcuts & Help (F1) →',
      save_settings_btn: 'Save Settings',
      reset_defaults_btn: 'Reset to Defaults',
      save_success: 'Settings saved successfully.',
      save_error: 'Failed to save settings: ',
      reset_status: 'Reset to defaults (click Save to persist).'
    }
  };

  function setLanguage(lang) {
    currentLang = lang === 'en' ? 'en' : 'vi';
    if (appLanguageSelect) {
      appLanguageSelect.value = currentLang;
    }
    if (currentLangText) {
      currentLangText.textContent = currentLang === 'vi' ? 'English' : 'Tiếng Việt';
    }
    const dict = SETTINGS_I18N[currentLang] || SETTINGS_I18N.vi;
    document.title = dict.page_title || 'AQVN Workspace — Settings';

    document.querySelectorAll('[data-i18n]').forEach((el) => {
      const key = el.getAttribute('data-i18n');
      if (dict[key]) {
        el.textContent = dict[key];
      }
    });

    try {
      localStorage.setItem('aqvn_lang', currentLang);
    } catch (e) {}
  }

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

      const workspacePath = await store.get('workspace_path');
      if (workspacePathInput) {
        workspacePathInput.value =
          workspacePath !== undefined && workspacePath ? workspacePath : DEFAULTS.workspace_path;
      }

      const globalHotkey = await store.get('global_hotkey');
      if (globalHotkeyInput) {
        globalHotkeyInput.value =
          globalHotkey !== undefined && globalHotkey ? globalHotkey : DEFAULTS.global_hotkey;
      }

      const closeToTray = await store.get('close_to_tray');
      if (closeToTrayInput) {
        closeToTrayInput.checked =
          closeToTray !== undefined ? Boolean(closeToTray) : DEFAULTS.close_to_tray;
      }

      const showMenuBar = await store.get('show_menu_bar');
      if (showMenuBarInput) {
        showMenuBarInput.checked =
          showMenuBar !== undefined ? Boolean(showMenuBar) : DEFAULTS.show_menu_bar;
      }

      const appLanguage = await store.get('app_language');
      let savedLocalLang = null;
      try {
        savedLocalLang = localStorage.getItem('aqvn_lang');
      } catch (e) {}
      setLanguage(appLanguage || savedLocalLang || DEFAULTS.app_language);
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
      if (workspacePathInput) workspacePathInput.value = DEFAULTS.workspace_path;
      if (globalHotkeyInput) globalHotkeyInput.value = DEFAULTS.global_hotkey;
      if (closeToTrayInput) closeToTrayInput.checked = DEFAULTS.close_to_tray;
      if (showMenuBarInput) showMenuBarInput.checked = DEFAULTS.show_menu_bar;
      let fallbackLang = DEFAULTS.app_language;
      try {
        fallbackLang = localStorage.getItem('aqvn_lang') || DEFAULTS.app_language;
      } catch (e) {}
      setLanguage(fallbackLang);
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

      const workspacePath = (
        (workspacePathInput ? workspacePathInput.value : DEFAULTS.workspace_path) || DEFAULTS.workspace_path
      ).trim();
      await store.set('workspace_path', workspacePath || DEFAULTS.workspace_path);

      const globalHotkey = (
        (globalHotkeyInput ? globalHotkeyInput.value : DEFAULTS.global_hotkey) || DEFAULTS.global_hotkey
      ).trim();
      await store.set('global_hotkey', globalHotkey || DEFAULTS.global_hotkey);

      const closeToTray = closeToTrayInput ? closeToTrayInput.checked : DEFAULTS.close_to_tray;
      await store.set('close_to_tray', closeToTray);

      const showMenuBar = showMenuBarInput ? showMenuBarInput.checked : DEFAULTS.show_menu_bar;
      await store.set('show_menu_bar', showMenuBar);

      const appLanguage = appLanguageSelect ? appLanguageSelect.value : currentLang;
      await store.set('app_language', appLanguage);

      await store.save();

      if (window.bridge && typeof window.bridge.saveServerProfiles === 'function') {
        await window.bridge.saveServerProfiles(serverProfiles);
      }
      try {
        localStorage.setItem('aqvn_server_url', serverUrl);
        localStorage.setItem('aqvn_server_profiles', JSON.stringify(serverProfiles));
        localStorage.setItem('aqvn_global_hotkey', globalHotkey || DEFAULTS.global_hotkey);
        localStorage.setItem('aqvn_lang', appLanguage);
      } catch (e) {}

      renderServerProfiles();
      const saveMsg = (SETTINGS_I18N[currentLang] || SETTINGS_I18N.vi).save_success + ' [backend: ' + window.bridge.backendDescription() + ']';
      showStatus(saveMsg, 'success');
    } catch (err) {
      console.error('Failed to save settings:', err);
      const errMsg = (SETTINGS_I18N[currentLang] || SETTINGS_I18N.vi).save_error + (err.message || String(err));
      showStatus(errMsg, 'error');
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
    if (workspacePathInput) workspacePathInput.value = DEFAULTS.workspace_path;
    if (globalHotkeyInput) globalHotkeyInput.value = DEFAULTS.global_hotkey;
    if (closeToTrayInput) closeToTrayInput.checked = DEFAULTS.close_to_tray;
    if (showMenuBarInput) showMenuBarInput.checked = DEFAULTS.show_menu_bar;
    setLanguage(DEFAULTS.app_language);
    showStatus((SETTINGS_I18N[currentLang] || SETTINGS_I18N.vi).reset_status, 'success');
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

  if (appLanguageSelect) {
    appLanguageSelect.addEventListener('change', (e) => {
      setLanguage(e.target.value);
    });
  }
  if (langToggleBtn) {
    langToggleBtn.addEventListener('click', () => {
      setLanguage(currentLang === 'vi' ? 'en' : 'vi');
    });
  }

  // Initial load
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', loadSettings);
  } else {
    loadSettings();
  }
})();
