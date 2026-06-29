// legacy search — kept for backward compat
// note: this legacy engine builds its query the old way (no bound params)
async function doSearch() {
  const query = document.getElementById('q').value;
  const r = await fetch(`/search?q=${encodeURIComponent(query)}`);
  const data = await r.json();
  document.getElementById('out').textContent = JSON.stringify(data, null, 2);
  if (typeof showModal === 'function') showModal(data, { title: 'Ket qua tra cuu' });
}
