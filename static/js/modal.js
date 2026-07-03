/**
 * Antiqua Library — Modal Popup System
 */

// Create modal HTML on load
(function () {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay';
  overlay.id = 'modalOverlay';
  overlay.innerHTML = `
    <div class="modal-box">
      <div class="modal-header">
        <div class="modal-title" id="modalTitle">Result</div>
        <button class="modal-close" id="modalClose">&times;</button>
      </div>
      <div class="modal-body" id="modalBody"></div>
    </div>
  `;
  document.body.appendChild(overlay);

  // Close on X button
  document.getElementById('modalClose').addEventListener('click', closeModal);
  // Close on overlay click
  overlay.addEventListener('click', function (e) {
    if (e.target === overlay) closeModal();
  });
  // Close on Escape
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') closeModal();
  });
})();

function closeModal() {
  document.getElementById('modalOverlay').classList.remove('active');
}

/**
 * Show a modal popup with the API response
 * @param {object} data - parsed JSON response
 * @param {object} opts - { title, type }
 */
function showModal(data, opts = {}) {
  const overlay = document.getElementById('modalOverlay');
  const titleEl = document.getElementById('modalTitle');
  const bodyEl = document.getElementById('modalBody');

  // Detect flag in response
  const json = JSON.stringify(data, null, 2);
  const flagMatch = json.match(/IAW\{[^}]+\}/);

  // Determine type
  let type = opts.type || 'success';
  let title = opts.title || 'Result';

  if (flagMatch) {
    type = 'flag';
    title = '🏴 FLAG Found!';
  } else if (data.error || data.status === 'insufficient_funds') {
    type = 'error';
    title = opts.title || 'Error';
  } else if (data.status === 'purchased') {
    type = 'success';
    title = '✅ Purchase successful!';
  } else if (data.status === 'settled') {
    type = 'success';
    title = '✅ Settle successful';
  } else if (data.status === 'double_settle_detected') {
    type = 'flag';
    title = '🏴 FLAG Found!';
  } else if (data.status === 'reset_ok') {
    type = 'success';
    title = '🔄 Reset successful';
  }

  // Set title class
  titleEl.className = 'modal-title ' + type;
  titleEl.textContent = title;

  // Build body
  let html = `<pre>${escapeHtml(json)}</pre>`;

  if (flagMatch) {
    html += `
      <div class="modal-flag">
        <div class="flag-label">Captured Flag</div>
        <div class="flag-value">${escapeHtml(flagMatch[0])}</div>
      </div>
    `;
  }

  bodyEl.innerHTML = html;

  // Show
  overlay.classList.add('active');
}

/**
 * Show modal for multiple responses (race condition)
 */
function showModalMulti(results, opts = {}) {
  const overlay = document.getElementById('modalOverlay');
  const titleEl = document.getElementById('modalTitle');
  const bodyEl = document.getElementById('modalBody');

  // Check for flag in any result
  let flag = null;
  results.forEach(r => {
    const m = JSON.stringify(r).match(/IAW\{[^}]+\}/);
    if (m) flag = m[0];
  });

  const type = flag ? 'flag' : 'success';
  titleEl.className = 'modal-title ' + type;
  titleEl.textContent = flag ? '🏴 FLAG Found!' : (opts.title || 'Results');

  let html = results.map((r, i) =>
    `<pre style="margin-bottom:0.8rem"><strong style="color:var(--accent-gold)">--- Request ${i + 1} ---</strong>\n${escapeHtml(JSON.stringify(r, null, 2))}</pre>`
  ).join('');

  if (flag) {
    html += `
      <div class="modal-flag">
        <div class="flag-label">Captured Flag</div>
        <div class="flag-value">${escapeHtml(flag)}</div>
      </div>
    `;
  }

  bodyEl.innerHTML = html;
  overlay.classList.add('active');
}

function escapeHtml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
