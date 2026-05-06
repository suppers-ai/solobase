// solobase files-browser bundle.
//
// Bootstrap JSON shape (server-rendered into <script type="application/json">):
//   { "bucket": "photos", "currentPrefix": "nested/" }
//
// All POST URLs match the existing /b/storage/api/* and /b/cloudstorage/* endpoints.
// `showToast` dispatches a `showToast` CustomEvent that the page-level toast handler
// (in ui/assets.rs::toast_js) listens for.
(function () {
  if (window.__solobaseFilesBrowserInit) return;
  window.__solobaseFilesBrowserInit = true;

  function readBootstrap() {
    const node = document.getElementById('files-browser-bootstrap');
    if (!node) return null;
    try {
      return JSON.parse(node.textContent || '{}');
    } catch (e) {
      return null;
    }
  }

  function showToast(message, type) {
    document.body.dispatchEvent(
      new CustomEvent('showToast', { detail: { message: message, type: type || 'info' } })
    );
  }

  function dragDropHandler(boot) {
    const root = document.querySelector('.page--list');
    if (!root || !boot.bucket) return;
    const bucket = boot.bucket;
    const prefix = boot.currentPrefix || '';

    root.addEventListener('dragenter', (e) => {
      e.preventDefault();
      root.classList.add('is-drop-target');
    });
    root.addEventListener('dragover', (e) => {
      e.preventDefault();
    });
    root.addEventListener('dragleave', (e) => {
      if (e.target === root) root.classList.remove('is-drop-target');
    });
    root.addEventListener('drop', async (e) => {
      e.preventDefault();
      root.classList.remove('is-drop-target');
      const files = Array.from(e.dataTransfer.files || []);
      if (files.length === 0) return;
      let successes = 0;
      let failures = 0;
      for (const f of files) {
        const key = prefix + f.name;
        const fd = new FormData();
        fd.append('file', f);
        const url =
          '/b/storage/api/buckets/' +
          encodeURIComponent(bucket) +
          '/objects?key=' +
          encodeURIComponent(key);
        try {
          const resp = await fetch(url, { method: 'POST', body: fd });
          if (resp.ok) {
            successes++;
          } else {
            failures++;
          }
        } catch (err) {
          failures++;
        }
      }
      if (successes > 0) {
        showToast(
          successes + ' uploaded' + (failures > 0 ? ', ' + failures + ' failed' : ''),
          failures > 0 ? 'error' : 'success'
        );
      } else {
        showToast(failures + ' upload failed', 'error');
      }
      window.location.reload();
    });
  }

  function bulkSelect() {
    const all = document.querySelector('[data-bulk-toggle]');
    if (!all) return;
    const rows = document.querySelectorAll('.bulk-select');
    all.addEventListener('change', () => {
      rows.forEach((r) => {
        r.checked = all.checked;
      });
      updateBulkBar();
    });
    rows.forEach((r) => r.addEventListener('change', updateBulkBar));
  }

  function selectedKeys() {
    return Array.from(document.querySelectorAll('.bulk-select:checked'))
      .map((c) => c.dataset.key)
      .filter(Boolean);
  }

  function updateBulkBar() {
    let bar = document.getElementById('bulk-action-bar');
    const keys = selectedKeys();
    if (!bar) {
      bar = document.createElement('div');
      bar.id = 'bulk-action-bar';
      bar.className = 'bulk-action-bar';
      bar.innerHTML = '<button type="button" data-bulk-delete>Delete selected</button>';
      const target = document.querySelector('.page--list .page-body');
      if (target) target.prepend(bar);
      bar.querySelector('[data-bulk-delete]').addEventListener('click', bulkDelete);
    }
    bar.style.display = keys.length > 0 ? '' : 'none';
    bar.dataset.count = String(keys.length);
  }

  async function bulkDelete() {
    const boot = readBootstrap() || {};
    const bucket = boot.bucket;
    const keys = selectedKeys();
    if (!bucket || !keys.length) return;
    if (!window.confirm('Delete ' + keys.length + ' file(s)?')) return;
    let failures = 0;
    for (const key of keys) {
      const url =
        '/b/storage/api/buckets/' +
        encodeURIComponent(bucket) +
        '/objects/' +
        encodeURIComponent(key);
      try {
        const resp = await fetch(url, { method: 'DELETE' });
        if (!resp.ok) failures++;
      } catch (e) {
        failures++;
      }
    }
    showToast(
      keys.length - failures + ' deleted' + (failures > 0 ? ', ' + failures + ' failed' : ''),
      failures > 0 ? 'error' : 'success'
    );
    window.location.reload();
  }

  function kebabMenu() {
    document.addEventListener('click', (e) => {
      const trigger = e.target.closest('[data-action-menu]');
      if (trigger) {
        e.stopPropagation();
        openKebab(trigger);
        return;
      }
      closeAllKebabs();
    });
  }

  function closeAllKebabs() {
    document.querySelectorAll('.kebab-popup').forEach((p) => p.remove());
  }

  function openKebab(trigger) {
    closeAllKebabs();
    const popup = document.createElement('div');
    popup.className = 'kebab-popup';
    if (trigger.dataset.token) {
      // Shares table kebab.
      popup.innerHTML = '<button type="button" data-action="revoke">Revoke share</button>';
      popup.querySelector('[data-action="revoke"]').addEventListener('click', () => {
        revokeShare(trigger.dataset.token);
      });
    } else if (trigger.dataset.key) {
      // Object table kebab.
      popup.innerHTML =
        '<button type="button" data-action="share">Share</button>' +
        '<button type="button" data-action="copy">Copy link</button>' +
        '<button type="button" data-action="delete">Delete</button>';
      popup.querySelector('[data-action="share"]').addEventListener('click', () => {
        shareModal(trigger.dataset.bucket, trigger.dataset.key);
      });
      popup.querySelector('[data-action="copy"]').addEventListener('click', () => {
        const url =
          window.location.origin +
          '/b/storage/api/buckets/' +
          encodeURIComponent(trigger.dataset.bucket) +
          '/objects/' +
          encodeURIComponent(trigger.dataset.key);
        navigator.clipboard.writeText(url);
        showToast('Link copied', 'success');
      });
      popup.querySelector('[data-action="delete"]').addEventListener('click', () => {
        confirmDelete(trigger.dataset.bucket, trigger.dataset.key);
      });
    }
    const rect = trigger.getBoundingClientRect();
    popup.style.position = 'fixed';
    popup.style.top = rect.bottom + 'px';
    popup.style.right = window.innerWidth - rect.right + 'px';
    document.body.appendChild(popup);
  }

  async function revokeShare(token) {
    if (!window.confirm('Revoke this share link?')) return;
    try {
      const resp = await fetch('/b/cloudstorage/shares/' + encodeURIComponent(token), {
        method: 'DELETE',
      });
      if (resp.ok) {
        showToast('Share revoked', 'success');
        window.location.reload();
      } else {
        showToast('Revoke failed', 'error');
      }
    } catch (e) {
      showToast('Revoke failed', 'error');
    }
  }

  function shareModal(bucket, key) {
    const dlg = document.createElement('dialog');
    dlg.className = 'share-modal';
    dlg.innerHTML =
      '<form method="dialog">' +
      '<h3>Create share link</h3>' +
      '<p><code></code></p>' +
      '<label>Expires in <select name="expires">' +
      '<option value="">Never</option>' +
      '<option value="1">1 day</option>' +
      '<option value="7" selected>7 days</option>' +
      '<option value="30">30 days</option>' +
      '</select></label>' +
      '<label>Max accesses <input name="max" type="number" min="0" placeholder="∞" /></label>' +
      '<div class="modal-actions">' +
      '<button type="button" data-action="cancel">Cancel</button>' +
      '<button type="button" data-action="create">Create</button>' +
      '</div>' +
      '</form>';
    // Set the source code element via textContent (avoids innerHTML XSS on bucket/key).
    dlg.querySelector('code').textContent = bucket + '/' + key;
    document.body.appendChild(dlg);
    dlg.showModal();
    dlg.querySelector('[data-action="cancel"]').addEventListener('click', () => {
      dlg.close();
      dlg.remove();
    });
    dlg.querySelector('[data-action="create"]').addEventListener('click', async () => {
      const days = dlg.querySelector('select[name="expires"]').value;
      const max = dlg.querySelector('input[name="max"]').value;
      const body = { bucket: bucket, key: key };
      if (days) body.expires_days = Number(days);
      if (max) body.max_access_count = Number(max);
      try {
        const resp = await fetch('/b/cloudstorage/shares', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        if (resp.ok) {
          const json = await resp.json();
          const url = window.location.origin + '/b/storage/direct/' + json.token;
          await navigator.clipboard.writeText(url);
          showToast('Share link copied', 'success');
          dlg.close();
          dlg.remove();
        } else {
          showToast('Share creation failed', 'error');
        }
      } catch (e) {
        showToast('Share creation failed', 'error');
      }
    });
  }

  async function confirmDelete(bucket, key) {
    if (!window.confirm('Delete ' + key + '?')) return;
    const url =
      '/b/storage/api/buckets/' +
      encodeURIComponent(bucket) +
      '/objects/' +
      encodeURIComponent(key);
    try {
      const resp = await fetch(url, { method: 'DELETE' });
      if (resp.ok) {
        showToast('Deleted', 'success');
        window.location.reload();
      } else {
        showToast('Delete failed', 'error');
      }
    } catch (e) {
      showToast('Delete failed', 'error');
    }
  }

  window.solobaseFilesBrowser = {
    init: function () {
      const boot = readBootstrap();
      // shareModal/kebab still useful even without bootstrap (e.g., shares page).
      kebabMenu();
      if (!boot) return;
      dragDropHandler(boot);
      bulkSelect();
    },
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => window.solobaseFilesBrowser.init());
  } else {
    window.solobaseFilesBrowser.init();
  }
})();
