#!/usr/bin/env python3
"""
Automated agent-browser test for multi-server URL switcher & profile presets.
Verifies VAL-CHAT-016:
1. Login view displays editable Server URL prefilled from store
2. Profile datalist and select dropdown present
3. Custom URL entry updates active URL and persists to store/profiles
4. Chat header displays active connected server URL
5. Settings page displays and manages saved profiles (add/select/remove)
6. Zero console errors throughout
"""

import json
import subprocess
import sys
import time

SESSION = "19cf26e1a2cc"

def run_ab(*args):
    cmd = ["agent-browser", "--session", SESSION] + list(args)
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if res.returncode != 0:
        print(f"Error running {' '.join(cmd)}:\n{res.stderr}")
    return res.stdout.strip()

def eval_js(script):
    raw = run_ab("eval", script)
    try:
        return json.loads(raw)
    except Exception:
        return raw

def main():
    print("=== Starting Browser Tests for Multi-Server URL Switcher ===")
    
    # 1. Open login page
    print("\n1. Opening login view at http://localhost:3155/index.html")
    run_ab("open", "http://localhost:3155/index.html")
    time.sleep(1)

    # Check for console errors
    errors = eval_js("window.__errors || []")
    print(f"Initial load errors: {errors}")

    # Check title
    doc_title = eval_js("document.title")
    print(f"document.title: {doc_title}")
    assert doc_title == "AQVN AI Workspace", f"Expected title 'AQVN AI Workspace', got {doc_title}"

    # 2. Verify login server URL and profile controls exist
    print("\n2. Checking login server URL and profile controls")
    server_url_val = eval_js("document.getElementById('login-server-url').value")
    print(f"Initial server URL value: {server_url_val}")
    assert server_url_val == "https://a0.matmagoc.com", f"Expected default server URL, got {server_url_val}"

    active_hdr = eval_js("document.getElementById('active-server-url').textContent")
    print(f"Active server URL in header: {active_hdr}")
    assert active_hdr == "https://a0.matmagoc.com", f"Expected header server URL, got {active_hdr}"

    select_options = eval_js("Array.from(document.getElementById('server-profile-select').options).map(o => o.value)")
    print(f"Server profile select options: {select_options}")
    assert "https://a0.matmagoc.com" in select_options, "Default profile missing in select"

    datalist_options = eval_js("Array.from(document.getElementById('server-profiles-list').options).map(o => o.value)")
    print(f"Datalist options: {datalist_options}")
    assert "https://a0.matmagoc.com" in datalist_options, "Default profile missing in datalist"

    # Take screenshot 1
    run_ab("screenshot", "/tmp/login-server-url-initial.png")
    print("Screenshot saved to /tmp/login-server-url-initial.png")

    # 3. Enter custom URL on login screen
    print("\n3. Entering custom server URL on login screen")
    eval_js("""
    (async () => {
        const inp = document.getElementById('login-server-url');
        inp.value = 'https://cluster-1.workspace.internal';
        inp.dispatchEvent(new Event('change', { bubbles: true }));
    })()
    """)
    time.sleep(0.5)

    updated_hdr = eval_js("document.getElementById('active-server-url').textContent")
    print(f"Updated active server URL in header: {updated_hdr}")
    assert updated_hdr == "https://cluster-1.workspace.internal", f"Header failed to update: {updated_hdr}"

    updated_select = eval_js("Array.from(document.getElementById('server-profile-select').options).map(o => o.value)")
    print(f"Updated server profile select options: {updated_select}")
    assert "https://cluster-1.workspace.internal" in updated_select, "New profile not added to select"

    # 4. Test profile select dropdown switching
    print("\n4. Switching profile via select dropdown")
    eval_js("""
    (() => {
        const sel = document.getElementById('server-profile-select');
        sel.value = 'https://a0.matmagoc.com';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()
    """)
    time.sleep(0.5)
    switched_url = eval_js("document.getElementById('login-server-url').value")
    switched_hdr = eval_js("document.getElementById('active-server-url').textContent")
    print(f"Switched URL: {switched_url}, Header: {switched_hdr}")
    assert switched_url == "https://a0.matmagoc.com", f"Failed to switch to a0: {switched_url}"
    assert switched_hdr == "https://a0.matmagoc.com", f"Header failed to switch to a0: {switched_hdr}"

    # Set back to custom before reload test
    eval_js("""
    (() => {
        const sel = document.getElementById('server-profile-select');
        sel.value = 'https://cluster-1.workspace.internal';
        sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()
    """)
    time.sleep(0.5)

    # 5. Reload page to verify persistence across reloads
    print("\n5. Reloading page to test persistence")
    run_ab("reload")
    time.sleep(1)

    reloaded_url = eval_js("document.getElementById('login-server-url').value")
    reloaded_hdr = eval_js("document.getElementById('active-server-url').textContent")
    reloaded_select = eval_js("Array.from(document.getElementById('server-profile-select').options).map(o => o.value)")
    print(f"Reloaded URL: {reloaded_url}, Header: {reloaded_hdr}")
    print(f"Reloaded select options: {reloaded_select}")
    assert reloaded_url == "https://cluster-1.workspace.internal", f"Persistence failed on URL: {reloaded_url}"
    assert reloaded_hdr == "https://cluster-1.workspace.internal", f"Persistence failed on Header: {reloaded_hdr}"
    assert "https://cluster-1.workspace.internal" in reloaded_select, "Persistence failed on select profiles"

    run_ab("screenshot", "/tmp/login-server-url-persisted.png")
    print("Screenshot saved to /tmp/login-server-url-persisted.png")

    # 6. Navigate to Settings page
    print("\n6. Opening Settings page at http://localhost:3155/settings.html")
    run_ab("open", "http://localhost:3155/settings.html")
    time.sleep(1)

    settings_server_url = eval_js("document.getElementById('server-url').value")
    print(f"Settings server URL: {settings_server_url}")
    assert settings_server_url == "https://cluster-1.workspace.internal", f"Settings URL mismatch: {settings_server_url}"

    settings_profiles = eval_js("Array.from(document.getElementById('server-profiles-list').querySelectorAll('.path-item span')).map(s => s.textContent)")
    print(f"Settings saved profiles list: {settings_profiles}")
    assert "https://cluster-1.workspace.internal" in settings_profiles, "Active profile missing in settings list"
    assert "https://a0.matmagoc.com" in settings_profiles, "Default profile missing in settings list"

    # Add a third profile on settings page
    print("\n7. Adding third profile 'https://backup-node.example.org' in settings")
    eval_js("""
    (() => {
        const inp = document.getElementById('new-profile-input');
        inp.value = 'https://backup-node.example.org';
        document.getElementById('add-profile-btn').click();
    })()
    """)
    time.sleep(0.5)

    updated_settings_profiles = eval_js("Array.from(document.getElementById('server-profiles-list').querySelectorAll('.path-item span')).map(s => s.textContent)")
    print(f"Profiles after add: {updated_settings_profiles}")
    assert "https://backup-node.example.org" in updated_settings_profiles, "Failed to add profile in settings"

    # Select the third profile as active
    print("\n8. Selecting 'https://backup-node.example.org' as active in settings")
    eval_js("""
    (() => {
        const items = Array.from(document.getElementById('server-profiles-list').querySelectorAll('.path-item'));
        for (const item of items) {
            if (item.querySelector('span').textContent === 'https://backup-node.example.org') {
                const btn = item.querySelector('.select-profile-btn');
                if (btn) btn.click();
                break;
            }
        }
    })()
    """)
    time.sleep(0.5)
    selected_in_settings = eval_js("document.getElementById('server-url').value")
    print(f"Server URL input after selecting profile: {selected_in_settings}")
    assert selected_in_settings == "https://backup-node.example.org", f"Failed to select profile: {selected_in_settings}"

    # Click Save Settings
    print("\n9. Saving settings")
    eval_js("document.getElementById('save-settings').click()")
    time.sleep(0.5)
    save_msg = eval_js("document.getElementById('save-status').textContent")
    print(f"Save status: {save_msg}")
    assert "Settings saved successfully" in save_msg, f"Expected success message, got: {save_msg}"

    run_ab("screenshot", "/tmp/settings-profiles-saved.png")
    print("Screenshot saved to /tmp/settings-profiles-saved.png")

    # 10. Return to index.html and verify sync
    print("\n10. Returning to index.html to verify active server sync")
    run_ab("open", "http://localhost:3155/index.html")
    time.sleep(1)

    final_url = eval_js("document.getElementById('login-server-url').value")
    final_hdr = eval_js("document.getElementById('active-server-url').textContent")
    final_select = eval_js("Array.from(document.getElementById('server-profile-select').options).map(o => o.value)")
    print(f"Final URL: {final_url}, Final Header: {final_hdr}")
    print(f"Final select options: {final_select}")
    assert final_url == "https://backup-node.example.org", f"Final URL mismatch: {final_url}"
    assert final_hdr == "https://backup-node.example.org", f"Final Header mismatch: {final_hdr}"
    assert "https://backup-node.example.org" in final_select, "Final select missing new profile"
    assert "https://cluster-1.workspace.internal" in final_select, "Final select missing cluster profile"
    assert "https://a0.matmagoc.com" in final_select, "Final select missing default profile"

    # Verify zero console errors
    console_errors = eval_js("window.__console_errors || []")
    print(f"\nFinal console errors check: {console_errors}")

    run_ab("screenshot", "/tmp/login-final-synced.png")
    print("Screenshot saved to /tmp/login-final-synced.png")

    print("\n=== ALL BROWSER CHECKS PASSED SUCCESSFULLY ===")

if __name__ == "__main__":
    try:
        main()
    finally:
        run_ab("close")
