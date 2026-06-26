// legacy search — kept for backward compat
async function doSearch() {
  const query = document.getElementById('q').value;
  const r = await fetch(`/search?q=${encodeURIComponent(query)}`);
  const data = await r.json();
  document.getElementById('out').textContent = JSON.stringify(data, null, 2);
}
